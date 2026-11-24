use std::fmt;
use std::time::Duration;

/// Formal error hierarchy for the SECE sandbox runtime and platform providers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxError {
    /// Strict enforcement requested, but host or execution context lacks mandatory verified isolation.
    StrictInvariantViolation {
        missing_invariant: String,
        platform_state: String,
        remediation_hint: String,
    },
    /// Candidate execution exceeded its wall-clock deadline.
    TimeoutExceeded {
        configured_limit: Duration,
        elapsed: Duration,
    },
    /// Candidate exceeded configured memory allocation ceiling (cgroup OOM or Job Object limit).
    MemoryExceeded {
        ceiling_bytes: usize,
        observed_bytes: Option<usize>,
        evidence: String,
    },
    /// Active process/thread count exceeded fork limit.
    ProcessLimitExceeded { max_pids: u32 },
    /// Output stream exceeded allocated byte budget.
    StreamOverflow {
        stream_name: &'static str,
        limit_bytes: usize,
    },
    /// Network isolation probe detected an interface, route, or live loopback state.
    NetworkBoundaryCompromised(String),
    /// Platform primitive failed during pre-execution setup or policy installation.
    PlatformSetupFailure(String),
    /// Explicit teardown or cgroup/process-tree cleanup failure (never swallowed silently).
    TeardownFailure(String),
    /// Candidate compilation or linking failed.
    CompilationFailure(String),
    /// Underlying OS I/O error.
    IoFailure(String),
}

impl fmt::Display for SandboxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StrictInvariantViolation {
                missing_invariant,
                platform_state,
                remediation_hint,
            } => write!(
                f,
                "Sandbox Strict Invariant Violation: '{missing_invariant}' is {platform_state}. {remediation_hint}"
            ),
            Self::TimeoutExceeded {
                configured_limit,
                elapsed,
            } => write!(
                f,
                "Candidate execution timed out after {elapsed:?} (limit: {configured_limit:?})"
            ),
            Self::MemoryExceeded {
                ceiling_bytes,
                observed_bytes,
                evidence,
            } => write!(
                f,
                "Candidate exceeded memory ceiling of {ceiling_bytes} bytes (observed: {observed_bytes:?}): {evidence}"
            ),
            Self::ProcessLimitExceeded { max_pids } => {
                write!(f, "Candidate exceeded maximum allowed process count ({max_pids})")
            }
            Self::StreamOverflow {
                stream_name,
                limit_bytes,
            } => write!(
                f,
                "Candidate {stream_name} exceeded output limit of {limit_bytes} bytes"
            ),
            Self::NetworkBoundaryCompromised(msg) => {
                write!(f, "Sandbox network boundary compromised: {msg}")
            }
            Self::PlatformSetupFailure(msg) => write!(f, "Sandbox platform setup failed: {msg}"),
            Self::TeardownFailure(msg) => write!(f, "Sandbox teardown failed: {msg}"),
            Self::CompilationFailure(msg) => write!(f, "Candidate compilation failed: {msg}"),
            Self::IoFailure(msg) => write!(f, "Sandbox I/O error: {msg}"),
        }
    }
}

impl std::error::Error for SandboxError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_error_display() {
        let err = SandboxError::StrictInvariantViolation {
            missing_invariant: "fs.mount_isolation".to_string(),
            platform_state: "Unsupported".to_string(),
            remediation_hint: "Run on Linux Tier 1 host.".to_string(),
        };
        let s = err.to_string();
        assert!(s.contains("fs.mount_isolation"));
        assert!(s.contains("Unsupported"));
        assert!(s.contains("Linux Tier 1"));
    }
}
