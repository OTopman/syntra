use crate::value::Value;
use std::collections::HashMap;
use std::time::Instant;

/// Reason triggering deoptimization to baseline (Specification 06 Section 6)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeoptReason {
    GuardViolation(String),
    UnexpectedType(String),
    NumericOverflow,
    NullDereferenceAttempted,
    PolymorphicCallsite,
    MemoryLimitExceeded,
    SimulatedFaultInjection,
}

impl std::fmt::Display for DeoptReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GuardViolation(s) => write!(f, "Guard violation: {s}"),
            Self::UnexpectedType(s) => write!(f, "Unexpected type: {s}"),
            Self::NumericOverflow => write!(f, "Numeric overflow detected"),
            Self::NullDereferenceAttempted => write!(f, "Null dereference trapped"),
            Self::PolymorphicCallsite => write!(f, "Polymorphic call site receiver mismatch"),
            Self::MemoryLimitExceeded => write!(f, "Memory watermark limit exceeded"),
            Self::SimulatedFaultInjection => write!(f, "Fault injection triggered"),
        }
    }
}

/// Continuation state reconstructed during deoptimization to safely resume baseline execution
#[derive(Debug, Clone, PartialEq)]
pub struct ContinuationFrame {
    pub function_name: String,
    pub resume_pc: usize,
    pub locals: HashMap<String, Value>,
    pub stack_operands: Vec<Value>,
}

impl ContinuationFrame {
    pub fn new(function_name: &str, resume_pc: usize) -> Self {
        Self {
            function_name: function_name.to_string(),
            resume_pc,
            locals: HashMap::new(),
            stack_operands: Vec::new(),
        }
    }

    pub fn set_local(&mut self, name: &str, val: Value) {
        self.locals.insert(name.to_string(), val);
    }

    pub fn push_operand(&mut self, val: Value) {
        self.stack_operands.push(val);
    }
}

/// Record of a deoptimization event for auditing and adaptation decisions
#[derive(Debug, Clone)]
pub struct DeoptEvent {
    pub timestamp: Instant,
    pub function: String,
    pub strategy: String,
    pub reason: DeoptReason,
    pub resume_frame: ContinuationFrame,
}

/// Deoptimization Manager (Specification 06 Section 6)
#[derive(Debug)]
pub struct DeoptManager {
    events: Vec<DeoptEvent>,
    deopt_counts: HashMap<String, usize>, // Function -> deopt frequency
    blacklist: HashMap<String, String>,   // Function -> retirement reason
    max_allowed_deopts: usize,
}

impl Default for DeoptManager {
    fn default() -> Self {
        Self::new(3) // Blacklist after 3 deopts to prevent deopt thrashing
    }
}

impl DeoptManager {
    pub fn new(max_allowed_deopts: usize) -> Self {
        Self {
            events: Vec::new(),
            deopt_counts: HashMap::new(),
            blacklist: HashMap::new(),
            max_allowed_deopts,
        }
    }

    pub fn is_blacklisted(&self, function: &str) -> bool {
        self.blacklist.contains_key(function)
    }

    pub fn get_blacklist_reason(&self, function: &str) -> Option<&str> {
        self.blacklist.get(function).map(|s| s.as_str())
    }

    pub fn events(&self) -> &[DeoptEvent] {
        &self.events
    }

    /// Triggers deoptimization: records event, reconstructs state, and checks anti-thrashing blacklist
    pub fn trigger_deopt(
        &mut self,
        function: &str,
        strategy: &str,
        reason: DeoptReason,
        resume_frame: ContinuationFrame,
    ) -> bool {
        let count = self.deopt_counts.entry(function.to_string()).or_insert(0);
        *count += 1;

        let should_retire = *count >= self.max_allowed_deopts;
        if should_retire && !self.blacklist.contains_key(function) {
            self.blacklist.insert(
                function.to_string(),
                format!(
                    "Deopt count ({count}) reached threshold; blacklisted to prevent compilation thrashing (Specification 06)"
                ),
            );
        }

        self.events.push(DeoptEvent {
            timestamp: Instant::now(),
            function: function.to_string(),
            strategy: strategy.to_string(),
            reason,
            resume_frame,
        });

        should_retire
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deopt_state_reconstruction_and_blacklisting() {
        let mut dm = DeoptManager::new(2);

        let mut frame = ContinuationFrame::new("process_batch", 4);
        frame.set_local("counter", Value::Int(42));
        frame.push_operand(Value::String("partial_payload".to_string()));

        // First deopt
        let retire1 = dm.trigger_deopt(
            "process_batch",
            "VectorizedOpt",
            DeoptReason::NumericOverflow,
            frame.clone(),
        );
        assert!(!retire1, "Should not retire on first deopt");
        assert!(!dm.is_blacklisted("process_batch"));

        // Second deopt reaches threshold 2
        let retire2 = dm.trigger_deopt(
            "process_batch",
            "VectorizedOpt",
            DeoptReason::NumericOverflow,
            frame,
        );
        assert!(
            retire2,
            "Should retire on reaching max_allowed_deopts threshold"
        );
        assert!(dm.is_blacklisted("process_batch"));
        assert!(dm
            .get_blacklist_reason("process_batch")
            .unwrap()
            .contains("blacklisted"));
    }
}
