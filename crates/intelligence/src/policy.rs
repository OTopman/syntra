use crate::planner::CompositePlan;
use std::collections::HashSet;

/// Developer control modes governing autonomous optimization (Specification 04, Section 5)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyMode {
    /// Analysis and recommendations only (read-only)
    Observe,
    /// Isolated benchmark execution without modifying production build
    Experiment,
    /// Explicit approval record required before application (Production default)
    Approve,
    /// Apply previously verified strategies automatically within policy constraints
    Enforce,
}

/// Verdict returned by the Policy Manager
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    Authorized,
    RequiresApproval { reason: String },
    Rejected { reason: String },
}

/// The Developer Policy Manager enforcing budgets, modes, and constraints
#[derive(Debug, Clone)]
pub struct PolicyManager {
    pub mode: PolicyMode,
    pub max_compile_time_ms: u64,
    pub max_memory_overhead_pct: f64,
    pub allowed_strategies: HashSet<String>,
    pub forbidden_strategies: HashSet<String>,
}

impl Default for PolicyManager {
    fn default() -> Self {
        Self {
            mode: PolicyMode::Approve, // Normative production default
            max_compile_time_ms: 10_000,
            max_memory_overhead_pct: 25.0,
            allowed_strategies: HashSet::new(),
            forbidden_strategies: HashSet::new(),
        }
    }
}

impl PolicyManager {
    pub fn new(mode: PolicyMode) -> Self {
        Self {
            mode,
            ..Default::default()
        }
    }

    /// Authorize or reject a composite optimization plan
    pub fn evaluate_plan(&self, plan: &CompositePlan) -> PolicyDecision {
        // Check for forbidden strategies
        for s in &plan.strategies {
            if self.forbidden_strategies.contains(&s.id) {
                return PolicyDecision::Rejected {
                    reason: format!(
                        "Strategy '{}' is explicitly forbidden by developer policy.",
                        s.id
                    ),
                };
            }
        }

        match self.mode {
            PolicyMode::Observe => PolicyDecision::Rejected {
                reason: "Policy mode is 'Observe' (Recommendations only, application disabled)."
                    .to_string(),
            },
            PolicyMode::Experiment => PolicyDecision::Authorized,
            PolicyMode::Approve => {
                // If every strategy is explicitly in allowed_strategies, authorize; otherwise require approval
                let unapproved: Vec<_> = plan
                    .strategies
                    .iter()
                    .filter(|s| !self.allowed_strategies.contains(&s.id))
                    .map(|s| s.id.clone())
                    .collect();

                if unapproved.is_empty() {
                    PolicyDecision::Authorized
                } else {
                    PolicyDecision::RequiresApproval {
                        reason: format!(
                            "Policy mode 'Approve' requires explicit approval for: {}",
                            unapproved.join(", ")
                        ),
                    }
                }
            }
            PolicyMode::Enforce => {
                // Only strictly allowed strategies may proceed
                let unallowed: Vec<_> = plan
                    .strategies
                    .iter()
                    .filter(|s| !self.allowed_strategies.contains(&s.id))
                    .map(|s| s.id.clone())
                    .collect();

                if unallowed.is_empty() {
                    PolicyDecision::Authorized
                } else {
                    PolicyDecision::Rejected {
                        reason: format!(
                            "Policy mode 'Enforce' rejected non-enforced strategies: {}",
                            unallowed.join(", ")
                        ),
                    }
                }
            }
        }
    }
}
