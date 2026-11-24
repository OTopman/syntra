use crate::capabilities::{
    CapabilityEvidenceRecord, CapabilityVerificationStatus, CleanupVerification,
    PlatformCapabilities,
};
use crate::config::{EnforcementMode, SandboxConfig};
use crate::error::SandboxError;
use crate::traits::{IsolatedChild, SandboxProvider};
use std::fs;
use std::io::Write;
use std::os::unix::io::{FromRawFd, IntoRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// Linux Tier 1 provider delegating candidate isolation to the sterile
/// `syntra-sandbox-supervisor` process (Deliverables 08B & 08C).
pub struct LinuxProvider {
    capabilities: PlatformCapabilities,
}

impl LinuxProvider {
    pub fn probe(execution_id: &str) -> Self {
        Self {
            capabilities: PlatformCapabilities::probe_host(execution_id),
        }
    }
}

impl SandboxProvider for LinuxProvider {
    fn name(&self) -> &'static str {
        "linux_two_tier_cgroup_ns"
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
        if config.enforcement_mode == EnforcementMode::Strict {
            self.capabilities.verify_strict_admissibility()?;
        } else {
            eprintln!(
                "[SECE Sandbox Running in Guarded Mode on Linux]\n\
                 Proceeding with best-effort isolation without failing on missing strict prerequisites."
            );
        }

        if !binary_path.is_file() {
            return Err(SandboxError::PlatformSetupFailure(format!(
                "Candidate binary does not exist: {}",
                binary_path.display()
            )));
        }

        let supervisor_bin = locate_tier1_supervisor()?;
        let spec_path = config.sandbox_base.join("tier1_spec.txt");
        let evidence_out_path = config.sandbox_base.join("tier1_evidence.txt");
        fs::create_dir_all(&config.sandbox_base).map_err(|e| {
            SandboxError::PlatformSetupFailure(format!(
                "Failed to create sandbox base '{}': {e}",
                config.sandbox_base.display()
            ))
        })?;

        write_supervisor_spec(&spec_path, binary_path, args, config, &evidence_out_path)?;

        // Spawn sterile Tier 1 supervisor process via execve (drops all Tier 0 CoW heap pages).
        let mut cmd = Command::new(&supervisor_bin);
        cmd.arg("--spec").arg(&spec_path);
        cmd.env_clear();
        cmd.env("LC_ALL", "C");
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        unsafe {
            cmd.pre_exec(|| {
                // Tier 1 dies immediately if Tier 0 terminates (Finding 1 Layer 1 & Layer 3).
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }

        let mut child = cmd.spawn().map_err(|e| {
            SandboxError::PlatformSetupFailure(format!(
                "Failed to spawn sterile Tier 1 supervisor '{}': {e}",
                supervisor_bin.display()
            ))
        })?;

        let ctrl_stdin = child.stdin.take();
        let stdout_fd = child
            .stdout
            .take()
            .map(|s| unsafe { OwnedFd::from_raw_fd(s.into_raw_fd()) });
        let stderr_fd = child
            .stderr
            .take()
            .map(|s| unsafe { OwnedFd::from_raw_fd(s.into_raw_fd()) });

        let cgroup_dir = resolve_cgroup_run_dir(&config.run_id);

        Ok(Box::new(LinuxTier1Handle {
            tier1_child: child,
            _ctrl_stdin: ctrl_stdin,
            stdout_fd,
            stderr_fd,
            cgroup_dir,
            evidence_out_path,
            sandbox_base: config.sandbox_base.clone(),
            config: config.clone(),
            base_caps: self.capabilities.clone(),
            reaped: false,
            exit_code: None,
        }))
    }

    fn cleanup_run(&self, run_id: &str) -> Result<(), SandboxError> {
        let cgroup_dir = resolve_cgroup_run_dir(run_id);
        if cgroup_dir.exists() {
            let _ = kill_cgroup_tree(&cgroup_dir);
            fs::remove_dir(&cgroup_dir).map_err(|e| {
                SandboxError::TeardownFailure(format!(
                    "Failed to remove cgroup '{}': {e}",
                    cgroup_dir.display()
                ))
            })?;
        }
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

struct LinuxTier1Handle {
    tier1_child: Child,
    // Closing this pipe triggers immediate Tier 1 teardown if Tier 0 drops abnormally.
    _ctrl_stdin: Option<std::process::ChildStdin>,
    stdout_fd: Option<OwnedFd>,
    stderr_fd: Option<OwnedFd>,
    cgroup_dir: PathBuf,
    evidence_out_path: PathBuf,
    sandbox_base: PathBuf,
    config: SandboxConfig,
    base_caps: PlatformCapabilities,
    reaped: bool,
    exit_code: Option<i32>,
}

impl IsolatedChild for LinuxTier1Handle {
    fn root_pid(&self) -> u32 {
        self.tier1_child.id()
    }

    fn try_wait(&mut self) -> Result<Option<i32>, SandboxError> {
        if self.reaped {
            return Ok(self.exit_code);
        }
        match self.tier1_child.try_wait() {
            Ok(Some(status)) => {
                self.reaped = true;
                self.exit_code = status.code();
                Ok(Some(status.code().unwrap_or(-1)))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(SandboxError::IoFailure(format!(
                "Tier 1 try_wait failed: {e}"
            ))),
        }
    }

    fn kill_tree(&mut self) -> Result<(), SandboxError> {
        let _ = kill_cgroup_tree(&self.cgroup_dir);
        if !self.reaped {
            let _ = self.tier1_child.kill();
            if let Ok(status) = self.tier1_child.wait() {
                self.reaped = true;
                self.exit_code = status.code();
            }
        }
        Ok(())
    }

    fn take_stdout_fd(&mut self) -> Option<OwnedFd> {
        self.stdout_fd.take()
    }

    fn take_stderr_fd(&mut self) -> Option<OwnedFd> {
        self.stderr_fd.take()
    }

    fn post_run_telemetry(&mut self) -> Option<SandboxError> {
        let events_path = self.cgroup_dir.join("memory.events");
        if let Ok(content) = fs::read_to_string(&events_path) {
            let mut oom_count: u64 = 0;
            for line in content.lines() {
                let mut parts = line.split_whitespace();
                if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
                    if matches!(k, "oom" | "oom_kill" | "oom_group_kill") {
                        oom_count += v.parse::<u64>().unwrap_or(0);
                    }
                }
            }
            if oom_count > 0 {
                let observed = fs::read_to_string(self.cgroup_dir.join("memory.peak"))
                    .ok()
                    .and_then(|s| s.trim().parse::<usize>().ok());
                return Some(SandboxError::MemoryExceeded {
                    ceiling_bytes: self.config.memory_allocation_ceiling_bytes,
                    observed_bytes: observed,
                    evidence: content.replace('\n', "; "),
                });
            }
        }
        None
    }

    fn capability_evidence(&self) -> Vec<CapabilityEvidenceRecord> {
        let mut caps = self.base_caps.clone();
        if let Ok(raw) = fs::read_to_string(&self.evidence_out_path) {
            for line in raw.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    match k {
                        "mem_cgroup" if v == "verified" => {
                            caps.mem_cgroup_accounting.status =
                                CapabilityVerificationStatus::EnforcedAndVerified;
                            caps.mem_cgroup_accounting
                                .kernel_observed_enforcement_evidence =
                                format!("cgroup={}", self.cgroup_dir.display());
                        }
                        "net_audit" if v == "verified" => {
                            caps.net_egress_isolation.status =
                                CapabilityVerificationStatus::EnforcedAndVerified;
                            caps.net_egress_isolation
                                .kernel_observed_enforcement_evidence =
                                "CLONE_NEWNET: lo DOWN, 0 routes".to_string();
                        }
                        "fs_pivot" if v == "verified" => {
                            caps.fs_mount_isolation.status =
                                CapabilityVerificationStatus::EnforcedAndVerified;
                            caps.fs_mount_isolation.kernel_observed_enforcement_evidence =
                                "pivot_root allowlist mount verified".to_string();
                        }
                        "seccomp" if v == "verified" => {
                            caps.seccomp_syscall_filtering.status =
                                CapabilityVerificationStatus::EnforcedAndVerified;
                            caps.seccomp_syscall_filtering
                                .kernel_observed_enforcement_evidence =
                                "PR_SET_NO_NEW_PRIVS + Seccomp-BPF installed before execve"
                                    .to_string();
                        }
                        "fd_sanitized" if v == "verified" => {
                            caps.fd_sanitization.status =
                                CapabilityVerificationStatus::EnforcedAndVerified;
                        }
                        _ => {}
                    }
                }
            }
        }
        caps.all_records()
    }

    fn verify_and_cleanup(&mut self) -> Result<CleanupVerification, SandboxError> {
        let mut surviving = Vec::new();
        if self.cgroup_dir.exists() {
            let _ = kill_cgroup_tree(&self.cgroup_dir);
            if let Ok(procs) = fs::read_to_string(self.cgroup_dir.join("cgroup.procs")) {
                for line in procs.lines() {
                    if let Ok(pid) = line.trim().parse::<u32>() {
                        surviving.push(pid);
                    }
                }
            }
            if !surviving.is_empty() {
                return Err(SandboxError::TeardownFailure(format!(
                    "Sandbox cgroup '{}' still contains surviving PIDs: {surviving:?}",
                    self.cgroup_dir.display()
                )));
            }
            if let Err(e) = fs::remove_dir(&self.cgroup_dir) {
                return Err(SandboxError::TeardownFailure(format!(
                    "Failed to remove sandbox cgroup '{}': {e}",
                    self.cgroup_dir.display()
                )));
            }
        }

        if !self.reaped {
            let _ = self.tier1_child.kill();
            let _ = self.tier1_child.wait();
            self.reaped = true;
        }

        Ok(CleanupVerification {
            cgroup_procs_empty: Some(surviving.is_empty()),
            cgroup_removed: Some(!self.cgroup_dir.exists()),
            temp_dir_removed: !self.sandbox_base.exists(),
            surviving_pids: surviving,
        })
    }
}

fn locate_tier1_supervisor() -> Result<PathBuf, SandboxError> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let direct = dir.join("syntra-sandbox-supervisor");
            if direct.is_file() {
                return Ok(direct);
            }
            if let Some(parent) = dir.parent() {
                let up = parent.join("syntra-sandbox-supervisor");
                if up.is_file() {
                    return Ok(up);
                }
            }
        }
    }
    Err(SandboxError::PlatformSetupFailure(
        "Sterile Tier 1 binary 'syntra-sandbox-supervisor' not found next to executable"
            .to_string(),
    ))
}

fn resolve_cgroup_run_dir(run_id: &str) -> PathBuf {
    let root = Path::new("/sys/fs/cgroup");
    if let Ok(self_cg) = fs::read_to_string("/proc/self/cgroup") {
        for line in self_cg.lines() {
            if let Some(rel) = line.strip_prefix("0::") {
                let parent = root.join(rel.trim_start_matches('/'));
                if parent.is_dir() {
                    return parent.join(format!("syntra_sece_{run_id}"));
                }
            }
        }
    }
    root.join("syntra_sandbox").join(run_id)
}

fn kill_cgroup_tree(cgroup_dir: &Path) -> Result<(), SandboxError> {
    let kill_file = cgroup_dir.join("cgroup.kill");
    if kill_file.exists() {
        fs::write(&kill_file, "1").map_err(|e| {
            SandboxError::TeardownFailure(format!("Failed writing cgroup.kill: {e}"))
        })?;
        return Ok(());
    }
    // Fallback freeze + iterative SIGKILL loop for pre-5.14 kernels.
    let freeze_file = cgroup_dir.join("cgroup.freeze");
    if freeze_file.exists() {
        let _ = fs::write(&freeze_file, "1");
    }
    for _ in 0..8 {
        let Ok(procs) = fs::read_to_string(cgroup_dir.join("cgroup.procs")) else {
            break;
        };
        let pids: Vec<libc::pid_t> = procs
            .lines()
            .filter_map(|l| l.trim().parse::<libc::pid_t>().ok())
            .collect();
        if pids.is_empty() {
            break;
        }
        for pid in pids {
            unsafe {
                let _ = libc::kill(pid, libc::SIGKILL);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    if freeze_file.exists() {
        let _ = fs::write(&freeze_file, "0");
    }
    Ok(())
}

fn write_supervisor_spec(
    spec_path: &Path,
    binary_path: &Path,
    args: &[String],
    config: &SandboxConfig,
    evidence_out: &Path,
) -> Result<(), SandboxError> {
    let mut file = fs::File::create(spec_path)
        .map_err(|e| SandboxError::PlatformSetupFailure(format!("Create spec failed: {e}")))?;
    writeln!(file, "run_id={}", config.run_id)
        .and_then(|_| writeln!(file, "binary={}", binary_path.display()))
        .and_then(|_| writeln!(file, "sandbox_base={}", config.sandbox_base.display()))
        .and_then(|_| writeln!(file, "evidence_out={}", evidence_out.display()))
        .and_then(|_| {
            writeln!(
                file,
                "memory_max={}",
                config.memory_allocation_ceiling_bytes
            )
        })
        .and_then(|_| writeln!(file, "pids_max={}", config.max_pids))
        .and_then(|_| writeln!(file, "cpu_quota={}", config.cpu_quota_percent))
        .and_then(|_| writeln!(file, "max_open_fds={}", config.max_open_fds))
        .and_then(|_| writeln!(file, "max_disk_bytes={}", config.max_disk_bytes))
        .and_then(|_| writeln!(file, "max_file_size={}", config.max_file_size_bytes))
        .and_then(|_| {
            writeln!(
                file,
                "strict={}",
                matches!(config.enforcement_mode, EnforcementMode::Strict)
            )
        })
        .and_then(|_| {
            for lib_path in &config.allow_read_only_lib_files {
                writeln!(file, "ro_lib={}", lib_path.display())?;
            }
            for (k, v) in &config.env_vars {
                writeln!(file, "env={k}={v}")?;
            }
            for a in args {
                writeln!(file, "arg={a}")?;
            }
            Ok(())
        })
        .map_err(|e| SandboxError::PlatformSetupFailure(format!("Write spec failed: {e}")))
}
