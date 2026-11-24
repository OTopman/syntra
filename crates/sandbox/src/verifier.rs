use crate::candidate::Candidate;
use crate::runner::RunOutput;

#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub is_valid: bool,
    pub air_invariants_passed: bool,
    pub differential_equivalence_passed: bool,
    pub failure_reason: Option<String>,
}

pub struct EquivalenceVerifier;

impl EquivalenceVerifier {
    /// Validates AIR invariants prior to compilation (Specification 05, ACE-001)
    pub fn verify_air_invariants(candidate: &Candidate) -> Result<(), String> {
        let diagnostics = syntra_air::verify_module(&candidate.air_module);
        if !diagnostics.is_empty() {
            let mut msgs = Vec::new();
            for d in diagnostics {
                msgs.push(format!("[{}] {}", d.code, d.message));
            }
            return Err(format!("AIR Invariant Violations: {}", msgs.join("; ")));
        }
        Ok(())
    }

    /// Performs differential testing comparing candidate execution with baseline (ACE-001, ACE-004)
    pub fn verify_differential_equivalence(
        candidate: &Candidate,
        baseline_output: &RunOutput,
        candidate_output: &RunOutput,
    ) -> VerificationResult {
        // 1. Check AIR invariants
        let air_invariants_passed = Self::verify_air_invariants(candidate).is_ok();
        if !air_invariants_passed {
            return VerificationResult {
                is_valid: false,
                air_invariants_passed: false,
                differential_equivalence_passed: false,
                failure_reason: Some("AIR invariant verification failed".to_string()),
            };
        }

        // 2. Check for candidate timeout
        if candidate_output.timed_out {
            return VerificationResult {
                is_valid: false,
                air_invariants_passed: true,
                differential_equivalence_passed: false,
                failure_reason: Some(format!(
                    "Candidate execution timed out (duration: {:?})",
                    candidate_output.duration
                )),
            };
        }

        // 3. Check exit code equivalence
        if candidate_output.exit_code != baseline_output.exit_code {
            return VerificationResult {
                is_valid: false,
                air_invariants_passed: true,
                differential_equivalence_passed: false,
                failure_reason: Some(format!(
                    "Exit code mismatch: expected {:?}, got {:?}",
                    baseline_output.exit_code, candidate_output.exit_code
                )),
            };
        }

        // 4. Check stdout equivalence byte-for-byte
        if candidate_output.stdout != baseline_output.stdout {
            return VerificationResult {
                is_valid: false,
                air_invariants_passed: true,
                differential_equivalence_passed: false,
                failure_reason: Some("Standard output diverged from baseline".to_string()),
            };
        }

        VerificationResult {
            is_valid: true,
            air_invariants_passed: true,
            differential_equivalence_passed: true,
            failure_reason: None,
        }
    }
}
