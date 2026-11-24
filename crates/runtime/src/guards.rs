use std::fmt;

/// Dynamic type tags for runtime type guard validation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeTag {
    Int,
    Float,
    Bool,
    String,
    Object,
    Array,
    Map,
    Null,
}

/// Dynamic runtime guard enforcing preconditions for specialized code (Specification 06, RUN-022)
#[derive(Debug, Clone, PartialEq)]
pub enum DynamicGuard {
    /// Asserts that argument `param_idx` matches expected type tag
    TypeGuard {
        param_idx: usize,
        expected_tag: TypeTag,
    },
    /// Asserts that integer argument is within expected bounds [min, max]
    RangeGuard {
        param_idx: usize,
        min_val: i64,
        max_val: i64,
    },
    /// Asserts that reference/object pointer is non-null
    NonNullGuard { param_idx: usize },
    /// Asserts that object receiver matches monomorphic class name
    MonomorphicReceiverGuard { expected_class: String },
    /// Asserts that loop iteration count does not exceed bound
    LoopBoundGuard { max_iterations: u64 },
    /// Asserts arbitrary boolean invariant predicate
    InvariantGuard {
        name: String,
        expected_boolean: bool,
    },
}

impl fmt::Display for DynamicGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TypeGuard {
                param_idx,
                expected_tag,
            } => {
                write!(f, "TypeGuard(arg[{param_idx}]: {expected_tag:?})")
            }
            Self::RangeGuard {
                param_idx,
                min_val,
                max_val,
            } => {
                write!(f, "RangeGuard(arg[{param_idx}]: [{min_val}..={max_val}])")
            }
            Self::NonNullGuard { param_idx } => {
                write!(f, "NonNullGuard(arg[{param_idx}])")
            }
            Self::MonomorphicReceiverGuard { expected_class } => {
                write!(f, "MonomorphicReceiverGuard(class == '{expected_class}')")
            }
            Self::LoopBoundGuard { max_iterations } => {
                write!(f, "LoopBoundGuard(iters <= {max_iterations})")
            }
            Self::InvariantGuard {
                name,
                expected_boolean,
            } => {
                write!(f, "InvariantGuard({name} == {expected_boolean})")
            }
        }
    }
}

/// Result of evaluating a set of dynamic guards
#[derive(Debug, Clone, PartialEq)]
pub enum GuardEvaluationResult {
    Passed,
    Violated {
        guard: DynamicGuard,
        observed_value: String,
    },
}

/// Guard Manager managing assumption validation for specialized functions (Specification 06)
#[derive(Debug, Default, Clone)]
pub struct GuardManager {
    guards: Vec<DynamicGuard>,
}

impl GuardManager {
    pub fn new() -> Self {
        Self { guards: Vec::new() }
    }

    pub fn with_guards(guards: Vec<DynamicGuard>) -> Self {
        Self { guards }
    }

    pub fn add_guard(&mut self, guard: DynamicGuard) {
        self.guards.push(guard);
    }

    pub fn guards(&self) -> &[DynamicGuard] {
        &self.guards
    }

    /// Evaluates integer range guard
    pub fn check_range(&self, param_idx: usize, val: i64) -> GuardEvaluationResult {
        for g in &self.guards {
            if let DynamicGuard::RangeGuard {
                param_idx: p,
                min_val,
                max_val,
            } = g
            {
                if *p == param_idx && (val < *min_val || val > *max_val) {
                    return GuardEvaluationResult::Violated {
                        guard: g.clone(),
                        observed_value: format!("{val}"),
                    };
                }
            }
        }
        GuardEvaluationResult::Passed
    }

    /// Evaluates non-null guard
    pub fn check_non_null(&self, param_idx: usize, is_null: bool) -> GuardEvaluationResult {
        for g in &self.guards {
            if let DynamicGuard::NonNullGuard { param_idx: p } = g {
                if *p == param_idx && is_null {
                    return GuardEvaluationResult::Violated {
                        guard: g.clone(),
                        observed_value: "null".to_string(),
                    };
                }
            }
        }
        GuardEvaluationResult::Passed
    }

    /// Evaluates monomorphic receiver class guard
    pub fn check_receiver(&self, actual_class: &str) -> GuardEvaluationResult {
        for g in &self.guards {
            if let DynamicGuard::MonomorphicReceiverGuard { expected_class } = g {
                if actual_class != expected_class {
                    return GuardEvaluationResult::Violated {
                        guard: g.clone(),
                        observed_value: actual_class.to_string(),
                    };
                }
            }
        }
        GuardEvaluationResult::Passed
    }

    /// Evaluates loop bound guard
    pub fn check_loop_bound(&self, iterations: u64) -> GuardEvaluationResult {
        for g in &self.guards {
            if let DynamicGuard::LoopBoundGuard { max_iterations } = g {
                if iterations > *max_iterations {
                    return GuardEvaluationResult::Violated {
                        guard: g.clone(),
                        observed_value: format!("{iterations} iterations"),
                    };
                }
            }
        }
        GuardEvaluationResult::Passed
    }

    /// Generates a deterministic hash fingerprint of all guards
    pub fn fingerprint(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.guards.len().hash(&mut hasher);
        for g in &self.guards {
            format!("{g:?}").hash(&mut hasher);
        }
        format!("{:016x}", hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_validation_success_and_violation() {
        let mut gm = GuardManager::new();
        gm.add_guard(DynamicGuard::RangeGuard {
            param_idx: 0,
            min_val: 0,
            max_val: 100,
        });
        gm.add_guard(DynamicGuard::NonNullGuard { param_idx: 1 });
        gm.add_guard(DynamicGuard::MonomorphicReceiverGuard {
            expected_class: "FastHttpHandler".to_string(),
        });

        // Test Range Pass
        assert_eq!(gm.check_range(0, 50), GuardEvaluationResult::Passed);
        // Test Range Fail
        match gm.check_range(0, 150) {
            GuardEvaluationResult::Violated { observed_value, .. } => {
                assert_eq!(observed_value, "150");
            }
            _ => panic!("Expected range violation"),
        }

        // Test Non-null
        assert_eq!(gm.check_non_null(1, false), GuardEvaluationResult::Passed);
        assert!(matches!(
            gm.check_non_null(1, true),
            GuardEvaluationResult::Violated { .. }
        ));

        // Test Monomorphic Receiver
        assert_eq!(
            gm.check_receiver("FastHttpHandler"),
            GuardEvaluationResult::Passed
        );
        assert!(matches!(
            gm.check_receiver("SlowFallbackHandler"),
            GuardEvaluationResult::Violated { .. }
        ));
    }
}
