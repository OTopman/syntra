use std::path::PathBuf;
use std::time::Duration;

/// Sandbox security enforcement policy.
///
/// Production builds intentionally exclude any unrestricted/disabled execution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnforcementMode {
    /// Zero-trust mode requiring verified kernel filesystem, network, process-tree,
    /// memory/swap, and descriptor isolation. Fails closed if any mandatory invariant
    /// cannot be verified.
    Strict,
    /// Cooperative resource/process-group supervision permitted only when explicitly
    /// selected by a developer for trusted local workloads. Emits an explicit warning
    /// when host isolation boundaries are absent.
    Guarded,
}

/// Independent byte budgets and drain bounds for candidate stdout and stderr.
#[derive(Debug, Clone)]
pub struct StreamBudgetConfig {
    /// Maximum captured bytes on stdout (default: 1 MiB).
    pub stdout_limit_bytes: usize,
    /// Maximum captured bytes on stderr (default: 1 MiB).
    pub stderr_limit_bytes: usize,
    /// Maximum bounded drain window after process termination (default: 500 ms).
    pub drain_timeout: Duration,
    /// If true, terminate candidate process tree immediately when either stream overflows.
    pub terminate_on_overflow: bool,
}

impl Default for StreamBudgetConfig {
    fn default() -> Self {
        Self {
            stdout_limit_bytes: 1024 * 1024,
            stderr_limit_bytes: 1024 * 1024,
            drain_timeout: Duration::from_millis(500),
            terminate_on_overflow: true,
        }
    }
}

/// Complete configuration for an isolated candidate execution.
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub run_id: String,
    pub sandbox_base: PathBuf,
    pub enforcement_mode: EnforcementMode,
    pub wall_clock_timeout: Duration,
    pub memory_allocation_ceiling_bytes: usize,
    pub max_pids: u32,
    pub max_open_fds: u64,
    pub max_disk_bytes: usize,
    pub max_file_size_bytes: u64,
    pub cpu_quota_percent: u32,
    pub streams: StreamBudgetConfig,
    pub env_vars: Vec<(String, String)>,
    pub allow_read_only_lib_files: Vec<PathBuf>,
}

impl SandboxConfig {
    /// Creates a default `Strict` configuration scoped to `/tmp/syntra_sandbox/<run_id>`.
    pub fn new(run_id: String) -> Self {
        let sandbox_base = std::env::temp_dir().join("syntra_sandbox").join(&run_id);
        Self {
            sandbox_base,
            run_id,
            enforcement_mode: EnforcementMode::Strict,
            wall_clock_timeout: Duration::from_secs(5),
            memory_allocation_ceiling_bytes: 128 * 1024 * 1024,
            max_pids: 32,
            max_open_fds: 64,
            max_disk_bytes: 32 * 1024 * 1024,
            max_file_size_bytes: 16 * 1024 * 1024,
            cpu_quota_percent: 100,
            streams: StreamBudgetConfig::default(),
            env_vars: Vec::new(),
            allow_read_only_lib_files: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_strict() {
        let cfg = SandboxConfig::new("unit_cfg_test".to_string());
        assert_eq!(cfg.enforcement_mode, EnforcementMode::Strict);
        assert_eq!(cfg.wall_clock_timeout, Duration::from_secs(5));
        assert_eq!(cfg.memory_allocation_ceiling_bytes, 128 * 1024 * 1024);
        assert_eq!(cfg.streams.drain_timeout, Duration::from_millis(500));
    }
}
