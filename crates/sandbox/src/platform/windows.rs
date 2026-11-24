use crate::capabilities::{
    CapabilityEvidenceRecord, CapabilityVerificationStatus, CleanupVerification,
    PlatformCapabilities,
};
use crate::config::{EnforcementMode, SandboxConfig};
use crate::error::SandboxError;
use crate::traits::{IsolatedChild, SandboxProvider};
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{FromRawHandle, RawHandle};
use std::path::{Path, PathBuf};
use std::ptr;
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, BOOL, FALSE, HANDLE, INVALID_HANDLE_VALUE, STILL_ACTIVE, TRUE,
    WAIT_OBJECT_0,
};
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_ACTIVE_PROCESSES, JOB_OBJECT_LIMIT_BREAKAWAY_OK, JOB_OBJECT_LIMIT_JOB_MEMORY,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK,
};
use windows_sys::Win32::System::Pipes::CreatePipe;
use windows_sys::Win32::System::Threading::{
    CreateProcessW, DeleteProcThreadAttributeList, GetExitCodeProcess,
    InitializeProcThreadAttributeList, ResumeThread, TerminateProcess, UpdateProcThreadAttribute,
    WaitForSingleObject, CREATE_NEW_PROCESS_GROUP, CREATE_SUSPENDED, EXTENDED_STARTUPINFO_PRESENT,
    PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_HANDLE_LIST, STARTF_USESTDHANDLES, STARTUPINFOEXW,
};

struct OwnedWinHandle(HANDLE);

impl OwnedWinHandle {
    fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedWinHandle {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}

/// Windows platform provider implementing Job Object resource containment in
/// `Guarded` mode and failing closed in `Strict` mode until a separate
/// AppContainer/WFP security isolation provider is implemented.
pub struct WindowsProvider {
    capabilities: PlatformCapabilities,
}

impl WindowsProvider {
    pub fn probe(execution_id: &str) -> Self {
        Self {
            capabilities: PlatformCapabilities::probe_host(execution_id),
        }
    }
}

impl SandboxProvider for WindowsProvider {
    fn name(&self) -> &'static str {
        "windows_job_guarded"
    }

    fn capabilities(&self) -> &PlatformCapabilities {
        &self.capabilities
    }

    fn spawn(
        &self,
        binary_path: &Path,
        args: &[String],
        config: &SandboxConfig,
    ) -> Result<Box<dyn IsolatedChild>, SandboxError> {
        // Strict mode unconditionally refuses until Windows Security Isolation Provider exists.
        if config.enforcement_mode == EnforcementMode::Strict {
            self.capabilities.verify_strict_admissibility()?;
            return Err(SandboxError::StrictInvariantViolation {
                missing_invariant: "fs.mount_isolation".to_string(),
                platform_state: "Unavailable".to_string(),
                remediation_hint: "Windows Job Objects provide Resource Containment only (no FS/network/DCOM broker confinement). Strict mode refused fail-closed."
                    .to_string(),
            });
        }

        if !binary_path.is_file() {
            return Err(SandboxError::PlatformSetupFailure(format!(
                "Candidate binary does not exist: {}",
                binary_path.display()
            )));
        }

        eprintln!(
            "[WARNING: SECE Sandbox Running in Guarded Mode on Windows]\n\
             Job Object enforces Resource Containment (memory, process cap, kill-on-close) only.\n\
             Filesystem, registry, network, and out-of-process COM/WMI broker spawns are NOT isolated.\n\
             Do NOT use Guarded mode for untrusted third-party candidates."
        );

        unsafe { spawn_windows_suspended_job(binary_path, args, config, &self.capabilities) }
    }

    fn cleanup_run(&self, run_id: &str) -> Result<(), SandboxError> {
        let base = std::env::temp_dir().join("syntra_sandbox").join(run_id);
        if base.exists() {
            fs::remove_dir_all(&base).map_err(|e| {
                SandboxError::TeardownFailure(format!(
                    "Failed to remove sandbox dir '{}': {e}",
                    base.display()
                ))
            })?;
        }
        Ok(())
    }
}

unsafe fn spawn_windows_suspended_job(
    binary_path: &Path,
    args: &[String],
    config: &SandboxConfig,
    base_caps: &PlatformCapabilities,
) -> Result<Box<dyn IsolatedChild>, SandboxError> {
    let h_job_raw = CreateJobObjectW(ptr::null(), ptr::null());
    if h_job_raw.is_null() {
        return Err(SandboxError::PlatformSetupFailure(format!(
            "CreateJobObjectW failed (Win32 error {})",
            GetLastError()
        )));
    }
    let h_job = OwnedWinHandle(h_job_raw);

    let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        | JOB_OBJECT_LIMIT_JOB_MEMORY
        | JOB_OBJECT_LIMIT_ACTIVE_PROCESSES;
    // Explicitly prohibit breakaway.
    limits.BasicLimitInformation.LimitFlags &= !JOB_OBJECT_LIMIT_BREAKAWAY_OK;
    limits.BasicLimitInformation.LimitFlags &= !JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK;
    limits.JobMemoryLimit = config.memory_allocation_ceiling_bytes;
    limits.BasicLimitInformation.ActiveProcessLimit = config.max_pids;

    let set_ok: BOOL = SetInformationJobObject(
        h_job.raw(),
        JobObjectExtendedLimitInformation,
        &limits as *const _ as *const _,
        std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
    );
    if set_ok == 0 {
        return Err(SandboxError::PlatformSetupFailure(format!(
            "SetInformationJobObject failed (Win32 error {})",
            GetLastError()
        )));
    }

    // Create inheritable stdout/stderr pipes and NUL stdin, strictly scoped via handle list.
    let sa = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: ptr::null_mut(),
        bInheritHandle: TRUE,
    };

    let (mut out_read, mut out_write) = (ptr::null_mut(), ptr::null_mut());
    if CreatePipe(&mut out_read, &mut out_write, &sa, 0) == 0 {
        return Err(SandboxError::IoFailure(format!(
            "CreatePipe(stdout) failed (Win32 error {})",
            GetLastError()
        )));
    }
    let out_read_guard = OwnedWinHandle(out_read);
    let out_write_guard = OwnedWinHandle(out_write);

    let (mut err_read, mut err_write) = (ptr::null_mut(), ptr::null_mut());
    if CreatePipe(&mut err_read, &mut err_write, &sa, 0) == 0 {
        return Err(SandboxError::IoFailure(format!(
            "CreatePipe(stderr) failed (Win32 error {})",
            GetLastError()
        )));
    }
    let err_read_guard = OwnedWinHandle(err_read);
    let err_write_guard = OwnedWinHandle(err_write);

    let mut attr_size: usize = 0;
    let _ = InitializeProcThreadAttributeList(ptr::null_mut(), 1, 0, &mut attr_size);
    let mut attr_buf = vec![0u8; attr_size.max(1)];
    let attr_list = attr_buf.as_mut_ptr() as *mut _;
    if InitializeProcThreadAttributeList(attr_list, 1, 0, &mut attr_size) == 0 {
        return Err(SandboxError::PlatformSetupFailure(format!(
            "InitializeProcThreadAttributeList failed (Win32 error {})",
            GetLastError()
        )));
    }

    let mut handle_list = [out_write_guard.raw(), err_write_guard.raw()];
    let upd_ok = UpdateProcThreadAttribute(
        attr_list,
        0,
        PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
        handle_list.as_mut_ptr() as *const _,
        std::mem::size_of_val(&handle_list),
        ptr::null_mut(),
        ptr::null(),
    );
    if upd_ok == 0 {
        DeleteProcThreadAttributeList(attr_list);
        return Err(SandboxError::PlatformSetupFailure(format!(
            "UpdateProcThreadAttribute failed (Win32 error {})",
            GetLastError()
        )));
    }

    let mut cmd_line = build_windows_command_line(binary_path, args);
    let mut si: STARTUPINFOEXW = std::mem::zeroed();
    si.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
    si.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    si.StartupInfo.hStdInput = INVALID_HANDLE_VALUE;
    si.StartupInfo.hStdOutput = out_write_guard.raw();
    si.StartupInfo.hStdError = err_write_guard.raw();
    si.lpAttributeList = attr_list;

    let mut pi: PROCESS_INFORMATION = std::mem::zeroed();
    let created = CreateProcessW(
        ptr::null(),
        cmd_line.as_mut_ptr(),
        ptr::null(),
        ptr::null(),
        TRUE, // Scoped strictly by PROC_THREAD_ATTRIBUTE_HANDLE_LIST
        CREATE_SUSPENDED | CREATE_NEW_PROCESS_GROUP | EXTENDED_STARTUPINFO_PRESENT,
        ptr::null(),
        ptr::null(),
        &si.StartupInfo,
        &mut pi,
    );
    DeleteProcThreadAttributeList(attr_list);

    if created == 0 {
        return Err(SandboxError::PlatformSetupFailure(format!(
            "CreateProcessW failed (Win32 error {})",
            GetLastError()
        )));
    }

    let h_proc = OwnedWinHandle(pi.hProcess);
    let h_thread = OwnedWinHandle(pi.hThread);

    // Assign to Job Object while suspended (0 candidate instructions executed).
    if AssignProcessToJobObject(h_job.raw(), h_proc.raw()) == 0 {
        let err = GetLastError();
        let _ = TerminateProcess(h_proc.raw(), 1);
        return Err(SandboxError::PlatformSetupFailure(format!(
            "AssignProcessToJobObject failed (Win32 error {err}); refusing execution outside Job"
        )));
    }

    let resume_rc = ResumeThread(h_thread.raw());
    if resume_rc == u32::MAX {
        let err = GetLastError();
        let _ = TerminateProcess(h_proc.raw(), 1);
        return Err(SandboxError::PlatformSetupFailure(format!(
            "ResumeThread failed (Win32 error {err})"
        )));
    }

    // Drop parent copies of write ends so readers observe EOF when process exits.
    drop(out_write_guard);
    drop(err_write_guard);

    let out_file = std::fs::File::from_raw_handle(out_read_guard.raw() as RawHandle);
    std::mem::forget(out_read_guard);
    let err_file = std::fs::File::from_raw_handle(err_read_guard.raw() as RawHandle);
    std::mem::forget(err_read_guard);

    let mut caps = base_caps.clone();
    caps.mem_cgroup_accounting.status = CapabilityVerificationStatus::ConfiguredUnverified;
    caps.mem_cgroup_accounting.configuration_result = format!(
        "CREATE_SUSPENDED -> AssignProcessToJobObject -> JobMemoryLimit={} -> ResumeThread",
        config.memory_allocation_ceiling_bytes
    );

    Ok(Box::new(WindowsChild {
        h_job,
        h_proc,
        pid: pi.dwProcessId,
        stdout_reader: Some(Box::new(out_file)),
        stderr_reader: Some(Box::new(err_file)),
        sandbox_base: config.sandbox_base.clone(),
        evidence: caps.all_records(),
        reaped: false,
        exit_code: None,
    }))
}

struct WindowsChild {
    h_job: OwnedWinHandle,
    h_proc: OwnedWinHandle,
    pid: u32,
    stdout_reader: Option<Box<dyn Read + Send>>,
    stderr_reader: Option<Box<dyn Read + Send>>,
    sandbox_base: PathBuf,
    evidence: Vec<CapabilityEvidenceRecord>,
    reaped: bool,
    exit_code: Option<i32>,
}

impl IsolatedChild for WindowsChild {
    fn root_pid(&self) -> u32 {
        self.pid
    }

    fn try_wait(&mut self) -> Result<Option<i32>, SandboxError> {
        if self.reaped {
            return Ok(self.exit_code);
        }
        unsafe {
            let wait = WaitForSingleObject(self.h_proc.raw(), 0);
            if wait == WAIT_OBJECT_0 {
                let mut code: u32 = 0;
                if GetExitCodeProcess(self.h_proc.raw(), &mut code) != 0 && code != STILL_ACTIVE {
                    self.reaped = true;
                    self.exit_code = Some(code as i32);
                    return Ok(self.exit_code);
                }
            }
        }
        Ok(None)
    }

    fn kill_tree(&mut self) -> Result<(), SandboxError> {
        unsafe {
            let _ = TerminateJobObject(self.h_job.raw(), 1);
            let _ = WaitForSingleObject(self.h_proc.raw(), 500);
        }
        self.reaped = true;
        Ok(())
    }

    fn take_stdout_reader(&mut self) -> Option<Box<dyn Read + Send>> {
        self.stdout_reader.take()
    }

    fn take_stderr_reader(&mut self) -> Option<Box<dyn Read + Send>> {
        self.stderr_reader.take()
    }

    fn capability_evidence(&self) -> Vec<CapabilityEvidenceRecord> {
        self.evidence.clone()
    }

    fn verify_and_cleanup(&mut self) -> Result<CleanupVerification, SandboxError> {
        let _ = self.kill_tree();
        Ok(CleanupVerification {
            cgroup_procs_empty: None,
            cgroup_removed: None,
            temp_dir_removed: !self.sandbox_base.exists(),
            surviving_pids: Vec::new(),
        })
    }
}

fn build_windows_command_line(binary: &Path, args: &[String]) -> Vec<u16> {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(format!("\"{}\"", binary.display()));
    for a in args {
        parts.push(format!("\"{}\"", a.replace('"', "\\\"")));
    }
    let joined = parts.join(" ");
    OsStr::new(&joined)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
