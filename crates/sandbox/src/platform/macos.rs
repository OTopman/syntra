use crate::capabilities::{
    CapabilityEvidenceRecord, CapabilityVerificationStatus, CleanupVerification,
    PlatformCapabilities,
};
use crate::config::{EnforcementMode, SandboxConfig};
use crate::error::SandboxError;
use crate::traits::{IsolatedChild, SandboxProvider};
use std::fs;
use std::os::unix::io::{FromRawFd, IntoRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// macOS platform provider implementing cooperative `Guarded` process supervision
/// and strict fail-closed refusal for `Strict` enforcement.
pub struct MacOsProvider {
    capabilities: PlatformCapabilities,
}

impl MacOsProvider {
    pub fn probe(execution_id: &str) -> Self {
        Self {
            capabilities: PlatformCapabilities::probe_host(execution_id),
        }
    }
}

impl SandboxProvider for MacOsProvider {
    fn name(&self) -> &'static str {
        "macos_guarded"
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
        // Strict mode is unconditionally refused on native macOS before spawning any process.
        if config.enforcement_mode == EnforcementMode::Strict {
            self.capabilities.verify_strict_admissibility()?;
            return Err(SandboxError::StrictInvariantViolation {
                missing_invariant: "fs.mount_isolation".to_string(),
                platform_state: "Unsupported".to_string(),
                remediation_hint: "Native macOS cannot satisfy Strict zero-trust isolation."
                    .to_string(),
            });
        }

        if !binary_path.is_file() {
            return Err(SandboxError::PlatformSetupFailure(format!(
                "Candidate binary does not exist: {}",
                binary_path.display()
            )));
        }

        // Guarded mode requires an explicit developer warning banner documenting absent boundaries.
        eprintln!(
            "[WARNING: SECE Sandbox Running in Guarded Mode on macOS Darwin]\n\
             Host platform lacks kernel filesystem, network, memory-tree, and setsid()-immune process-tree isolation.\n\
             Running candidate with best-effort process-group termination, rlimits, FD sanitization, and monotonic timeout only.\n\
             Do NOT use Guarded mode for untrusted third-party candidates."
        );

        let parent_dir = binary_path.parent().unwrap_or(&config.sandbox_base);
        let max_open_fds = config.max_open_fds;
        let max_file_size = config.max_file_size_bytes;
        let cpu_secs = config.wall_clock_timeout.as_secs().max(1);

        let mut cmd = Command::new(binary_path);
        cmd.args(args);
        cmd.env_clear();

        let system_path =
            std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin:/usr/local/bin".to_string());
        cmd.env("PATH", system_path);
        cmd.env("TMPDIR", parent_dir);
        cmd.env("HOME", parent_dir);
        cmd.env("USER", "syntra_sandbox");
        cmd.env("LC_ALL", "C");
        for (k, v) in &config.env_vars {
            cmd.env(k, v);
        }

        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        unsafe {
            cmd.pre_exec(move || {
                // Establish isolated session & detach any controlling terminal (blocks TIOCSTI).
                if libc::setsid() < 0 {
                    let _ = libc::setpgid(0, 0);
                }

                // Prevent background terminal reads/writes from suspending child (RISK-03).
                let _ = libc::signal(libc::SIGTTOU, libc::SIG_IGN);
                let _ = libc::signal(libc::SIGTTIN, libc::SIG_IGN);

                // Ensure FD 0 is unequivocally /dev/null.
                let dev_null = libc::open(c"/dev/null".as_ptr(), libc::O_RDONLY);
                if dev_null >= 0 {
                    if dev_null != 0 {
                        let _ = libc::dup2(dev_null, 0);
                        libc::close(dev_null);
                    }
                } else {
                    return Err(std::io::Error::last_os_error());
                }

                // Mark all inherited descriptors >= 3 with FD_CLOEXEC without breaking
                // Rust stdlib's internal exec-error pipe prior to execve.
                set_cloexec_above_stdio(max_open_fds.clamp(256, 4096) as libc::c_int);

                // Cooperative resource limits.
                let fsize = libc::rlimit {
                    rlim_cur: max_file_size,
                    rlim_max: max_file_size,
                };
                let _ = libc::setrlimit(libc::RLIMIT_FSIZE, &fsize);

                let cpu = libc::rlimit {
                    rlim_cur: cpu_secs + 1,
                    rlim_max: cpu_secs + 1,
                };
                let _ = libc::setrlimit(libc::RLIMIT_CPU, &cpu);

                let nofile = libc::rlimit {
                    rlim_cur: max_open_fds,
                    rlim_max: max_open_fds,
                };
                let _ = libc::setrlimit(libc::RLIMIT_NOFILE, &nofile);

                Ok(())
            });
        }

        let mut child = cmd.spawn().map_err(|e| {
            SandboxError::PlatformSetupFailure(format!(
                "Failed to spawn guarded binary '{}': {e}",
                binary_path.display()
            ))
        })?;

        let stdout_fd = child
            .stdout
            .take()
            .map(|s| unsafe { OwnedFd::from_raw_fd(s.into_raw_fd()) });
        let stderr_fd = child
            .stderr
            .take()
            .map(|s| unsafe { OwnedFd::from_raw_fd(s.into_raw_fd()) });

        let mut caps = self.capabilities.clone();
        caps.fd_sanitization.status = CapabilityVerificationStatus::ConfiguredUnverified;
        caps.fd_sanitization.configuration_result =
            "stdin -> /dev/null; setsid(); FD_CLOEXEC sweep on descriptors >= 3".to_string();
        caps.time_wall_clock_timeout.status = CapabilityVerificationStatus::ConfiguredUnverified;
        caps.time_wall_clock_timeout.configuration_result =
            format!("monotonic deadline {:?} active", config.wall_clock_timeout);

        let pid = child.id();
        Ok(Box::new(MacOsChild {
            child,
            pgid: pid,
            stdout_fd,
            stderr_fd,
            sandbox_base: config.sandbox_base.clone(),
            evidence: caps.all_records(),
            reaped: false,
            exit_code: None,
        }))
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

struct MacOsChild {
    child: Child,
    pgid: u32,
    stdout_fd: Option<OwnedFd>,
    stderr_fd: Option<OwnedFd>,
    sandbox_base: PathBuf,
    evidence: Vec<CapabilityEvidenceRecord>,
    reaped: bool,
    exit_code: Option<i32>,
}

impl IsolatedChild for MacOsChild {
    fn root_pid(&self) -> u32 {
        self.child.id()
    }

    fn try_wait(&mut self) -> Result<Option<i32>, SandboxError> {
        if self.reaped {
            return Ok(self.exit_code);
        }
        match self.child.try_wait() {
            Ok(Some(status)) => {
                self.reaped = true;
                self.exit_code = status.code();
                Ok(Some(status.code().unwrap_or(-1)))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(SandboxError::IoFailure(format!("try_wait failed: {e}"))),
        }
    }

    fn kill_tree(&mut self) -> Result<(), SandboxError> {
        // Best-effort negative-PGID kill followed by direct child kill & reap.
        unsafe {
            let _ = libc::kill(-(self.pgid as libc::pid_t), libc::SIGKILL);
        }
        if !self.reaped {
            let _ = self.child.kill();
            if let Ok(status) = self.child.wait() {
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

    fn capability_evidence(&self) -> Vec<CapabilityEvidenceRecord> {
        self.evidence.clone()
    }

    fn verify_and_cleanup(&mut self) -> Result<CleanupVerification, SandboxError> {
        // Always signal the entire process group unconditionally so background
        // descendants cannot outlive a normal exit(0) of the root child.
        let _ = self.kill_tree();
        let pid_alive = unsafe { libc::kill(self.pgid as libc::pid_t, 0) == 0 };
        let surviving = if pid_alive {
            vec![self.pgid]
        } else {
            Vec::new()
        };
        Ok(CleanupVerification {
            cgroup_procs_empty: None,
            cgroup_removed: None,
            temp_dir_removed: !self.sandbox_base.exists(),
            surviving_pids: surviving,
        })
    }
}

/// Marks all open descriptors in `[3..upper_bound]` with `FD_CLOEXEC` using only
/// async-signal-safe `fcntl` calls (strictly avoiding heap-allocating `opendir` inside `pre_exec`).
unsafe fn set_cloexec_above_stdio(upper_bound: libc::c_int) {
    for fd in 3..upper_bound {
        let flags = libc::fcntl(fd, libc::F_GETFD);
        if flags >= 0 {
            let _ = libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC);
        }
    }
}
