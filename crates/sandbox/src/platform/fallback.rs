use crate::capabilities::PlatformCapabilities;
use crate::config::SandboxConfig;
use crate::error::SandboxError;
use crate::traits::{IsolatedChild, SandboxProvider};
use std::path::Path;

/// Unconditionally fail-closed provider for unsupported host platforms.
pub struct FallbackProvider {
    capabilities: PlatformCapabilities,
}

impl FallbackProvider {
    pub fn probe(execution_id: &str) -> Self {
        Self {
            capabilities: PlatformCapabilities::probe_host(execution_id),
        }
    }
}

impl SandboxProvider for FallbackProvider {
    fn name(&self) -> &'static str {
        "fallback"
    }

    fn capabilities(&self) -> &PlatformCapabilities {
        &self.capabilities
    }

    fn spawn(
        &self,
        _binary_path: &Path,
        _args: &[String],
        _config: &SandboxConfig,
    ) -> Result<Box<dyn IsolatedChild>, SandboxError> {
        Err(SandboxError::StrictInvariantViolation {
            missing_invariant: "platform.sandbox_backend".to_string(),
            platform_state: "Unsupported".to_string(),
            remediation_hint: format!(
                "Host platform '{}' has no verified SECE isolation backend. Execution refused fail-closed.",
                std::env::consts::OS
            ),
        })
    }

    fn cleanup_run(&self, _run_id: &str) -> Result<(), SandboxError> {
        Ok(())
    }
}
