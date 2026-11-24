use crate::capabilities::{CapabilityEvidenceRecord, CleanupVerification, PlatformCapabilities};
use crate::config::SandboxConfig;
use crate::error::SandboxError;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::io::OwnedFd;

/// Active isolated candidate process tree managed by a platform provider.
pub trait IsolatedChild {
    /// Returns the OS PID of the supervised root child (or Tier 1 supervisor on Linux).
    fn root_pid(&self) -> u32;

    /// Non-blocking check for process completion.
    fn try_wait(&mut self) -> Result<Option<i32>, SandboxError>;

    /// Unconditionally terminates the entire candidate process tree and reaps direct zombies.
    fn kill_tree(&mut self) -> Result<(), SandboxError>;

    /// Takes ownership of the readable non-blocking stdout descriptor on Unix.
    #[cfg(unix)]
    fn take_stdout_fd(&mut self) -> Option<OwnedFd>;

    /// Takes ownership of the readable non-blocking stderr descriptor on Unix.
    #[cfg(unix)]
    fn take_stderr_fd(&mut self) -> Option<OwnedFd>;

    /// Takes ownership of a generic reader on non-Unix platforms.
    #[cfg(not(unix))]
    fn take_stdout_reader(&mut self) -> Option<Box<dyn std::io::Read + Send>>;

    /// Takes ownership of a generic reader on non-Unix platforms.
    #[cfg(not(unix))]
    fn take_stderr_reader(&mut self) -> Option<Box<dyn std::io::Read + Send>>;

    /// Records any kernel-observed resource or OOM telemetry after execution finishes.
    fn post_run_telemetry(&mut self) -> Option<SandboxError> {
        None
    }

    /// Returns updated per-execution capability evidence collected during spawn/run.
    fn capability_evidence(&self) -> Vec<CapabilityEvidenceRecord>;

    /// Verifies and executes final host resource cleanup (cgroup removal, empty process table).
    fn verify_and_cleanup(&mut self) -> Result<CleanupVerification, SandboxError>;
}

/// Platform-specific sandbox isolation backend.
pub trait SandboxProvider: Send + Sync {
    /// Human-readable backend identifier.
    fn name(&self) -> &'static str;

    /// Returns dynamically probed capabilities for this host.
    fn capabilities(&self) -> &PlatformCapabilities;

    /// Spawns a candidate binary under the configured enforcement mode and limits.
    fn spawn(
        &self,
        binary_path: &Path,
        args: &[String],
        config: &SandboxConfig,
    ) -> Result<Box<dyn IsolatedChild>, SandboxError>;

    /// Cleans up run-level resources on the host.
    fn cleanup_run(&self, run_id: &str) -> Result<(), SandboxError>;
}
