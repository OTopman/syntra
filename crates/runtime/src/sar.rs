use crate::code_cache::{CodeCache, CodeCacheEntry, SpecialisationIdentity};
use crate::deopt::{ContinuationFrame, DeoptManager, DeoptReason};
use crate::guards::{DynamicGuard, GuardEvaluationResult, GuardManager};
use crate::value::Value;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Actions taken by the Self-Adapting Runtime (SAR) Engine
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdaptationAction {
    Maintain,
    ScaleWorkers {
        target_workers: usize,
        reason: String,
    },
    AdjustMemoryPacing {
        recycle_frequency_ops: u64,
        reason: String,
    },
    ThrottleThroughput {
        delay_micros: u64,
        reason: String,
    },
    ActivateSpecialisation {
        function: String,
        strategy: String,
    },
    Deoptimise {
        function: String,
        fallback_reason: String,
    },
    RetireSpecialisation {
        function: String,
        reason: String,
    },
}

/// Dynamic resource metrics collected by SAR
#[derive(Debug, Clone)]
pub struct SarTelemetry {
    pub active_tasks: usize,
    pub queued_tasks: usize,
    pub allocated_bytes: u64,
    pub allocation_limit_bytes: u64,
    pub total_ops: u64,
    pub last_tick: Instant,
}

impl Default for SarTelemetry {
    fn default() -> Self {
        Self {
            active_tasks: 1,
            queued_tasks: 0,
            allocated_bytes: 0,
            allocation_limit_bytes: 1024 * 1024 * 1024, // 1 GB default limit
            total_ops: 0,
            last_tick: Instant::now(),
        }
    }
}

/// Request for background compilation without blocking live execution (Specification 06, RUN-021)
#[derive(Debug, Clone)]
pub struct SpecialisationRequest {
    pub function_name: String,
    pub strategy_id: String,
    pub air_digest: String,
    pub estimated_speedup_pct: f64,
    pub guards: Vec<DynamicGuard>,
}

/// Self-Adapting Runtime (SAR) Engine (Specification 06 & Specification 10 Phase 8)
#[derive(Debug)]
pub struct SarEngine {
    pub enabled: bool,
    pub telemetry: SarTelemetry,
    pub code_cache: CodeCache,
    pub guard_manager: GuardManager,
    pub deopt_manager: DeoptManager,
    pub background_queue: Vec<SpecialisationRequest>,
    pub history: Vec<AdaptationAction>,
    pub max_workers: usize,
    pub min_workers: usize,
    pub current_workers: usize,
    pub recycle_pacing_ops: u64,
}

impl Default for SarEngine {
    fn default() -> Self {
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self {
            enabled: false, // RUN-020: Disabled by default
            telemetry: SarTelemetry::default(),
            code_cache: CodeCache::new(64, 32 * 1024 * 1024), // 64 entries, 32MB max
            guard_manager: GuardManager::new(),
            deopt_manager: DeoptManager::new(3),
            background_queue: Vec::new(),
            history: Vec::new(),
            max_workers: cpus * 2,
            min_workers: 1,
            current_workers: cpus,
            recycle_pacing_ops: 10_000,
        }
    }
}

impl SarEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct an active engine explicitly permitted by policy or CLI flag (--sar)
    pub fn new_active() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Record task arrival or queue update
    pub fn record_task_queue(&mut self, active: usize, queued: usize) {
        self.telemetry.active_tasks = active;
        self.telemetry.queued_tasks = queued;
    }

    /// Record memory allocation tick
    pub fn record_allocation(&mut self, bytes: u64) {
        self.telemetry.allocated_bytes = self.telemetry.allocated_bytes.saturating_add(bytes);
        self.telemetry.total_ops = self.telemetry.total_ops.saturating_add(1);
    }

    /// Record memory release tick
    pub fn record_deallocation(&mut self, bytes: u64) {
        self.telemetry.allocated_bytes = self.telemetry.allocated_bytes.saturating_sub(bytes);
    }

    /// Submits a candidate specialization for asynchronous compilation (RUN-021)
    pub fn request_specialisation(&mut self, req: SpecialisationRequest) {
        if !self.enabled || self.deopt_manager.is_blacklisted(&req.function_name) {
            return;
        }
        self.background_queue.push(req);
    }

    /// Polls background compilation queue, activates completed specializations (RUN-021, RUN-024)
    pub fn poll_background_compilation(&mut self) -> Option<AdaptationAction> {
        if !self.enabled || self.background_queue.is_empty() {
            return None;
        }

        let req = self.background_queue.remove(0);
        let guard_fp = GuardManager::with_guards(req.guards.clone()).fingerprint();
        let target = std::env::consts::ARCH;

        let identity = SpecialisationIdentity::new(
            &req.function_name,
            &req.strategy_id,
            &req.air_digest,
            target,
            &guard_fp,
        );

        // Register code in bounded cache
        let entry = CodeCacheEntry::new(identity, 4096);
        let _ = self.code_cache.insert(entry);

        for g in req.guards {
            self.guard_manager.add_guard(g);
        }

        let action = AdaptationAction::ActivateSpecialisation {
            function: req.function_name.clone(),
            strategy: req.strategy_id.clone(),
        };
        self.history.push(action.clone());
        Some(action)
    }

    /// Verifies guards before entering specialized code; triggers deopt on assumption violation (RUN-022, RUN-023)
    pub fn verify_call_guards(
        &mut self,
        function: &str,
        strategy: &str,
        receiver_class: Option<&str>,
        args: &[Value],
        resume_pc: usize,
    ) -> Result<(), DeoptReason> {
        if !self.enabled {
            return Ok(());
        }

        // Check monomorphic receiver guard
        if let Some(actual_cls) = receiver_class {
            if let GuardEvaluationResult::Violated {
                guard,
                observed_value,
            } = self.guard_manager.check_receiver(actual_cls)
            {
                let reason = DeoptReason::GuardViolation(format!(
                    "Expected receiver guard {guard:?}, observed '{observed_value}'"
                ));
                self.trigger_deopt(function, strategy, reason.clone(), resume_pc);
                return Err(reason);
            }
        }

        // Check range guards for integer arguments
        for (idx, arg) in args.iter().enumerate() {
            if let Value::Int(n) = arg {
                if let GuardEvaluationResult::Violated {
                    guard,
                    observed_value,
                } = self.guard_manager.check_range(idx, *n)
                {
                    let reason = DeoptReason::GuardViolation(format!(
                        "Range guard {guard:?} violated with value '{observed_value}'"
                    ));
                    self.trigger_deopt(function, strategy, reason.clone(), resume_pc);
                    return Err(reason);
                }
            }
        }

        Ok(())
    }

    /// Triggers deoptimization, reconstructs continuation frame, and handles retirement
    pub fn trigger_deopt(
        &mut self,
        function: &str,
        strategy: &str,
        reason: DeoptReason,
        resume_pc: usize,
    ) {
        let frame = ContinuationFrame::new(function, resume_pc);
        let should_retire =
            self.deopt_manager
                .trigger_deopt(function, strategy, reason.clone(), frame);

        let deopt_action = AdaptationAction::Deoptimise {
            function: function.to_string(),
            fallback_reason: reason.to_string(),
        };
        self.history.push(deopt_action);

        if should_retire {
            let retire_action = AdaptationAction::RetireSpecialisation {
                function: function.to_string(),
                reason: "Repeated deopts triggered anti-thrashing retirement".to_string(),
            };
            self.history.push(retire_action);
        }
    }

    /// Injects a simulated fault to verify safe recovery and baseline fallback (Phase 8 Gate)
    pub fn inject_fault(&mut self, function: &str, strategy: &str) {
        self.trigger_deopt(function, strategy, DeoptReason::SimulatedFaultInjection, 0);
    }

    /// Evaluates current telemetry against safety and performance bounds
    pub fn evaluate(&mut self) -> AdaptationAction {
        if !self.enabled {
            return AdaptationAction::Maintain;
        }

        // 1. Process any pending background compilation
        if let Some(action) = self.poll_background_compilation() {
            return action;
        }

        // 2. Memory Pressure Governor: Check if approaching watermark (85% of limit)
        let usage_ratio = if self.telemetry.allocation_limit_bytes > 0 {
            self.telemetry.allocated_bytes as f64 / self.telemetry.allocation_limit_bytes as f64
        } else {
            0.0
        };

        if usage_ratio > 0.85 {
            let action = AdaptationAction::AdjustMemoryPacing {
                recycle_frequency_ops: 1_000, // Accelerate recycling 10x
                reason: format!(
                    "High memory watermark detected ({:.1}% used)",
                    usage_ratio * 100.0
                ),
            };
            self.history.push(action.clone());
            self.recycle_pacing_ops = 1_000;
            return action;
        }

        // 3. Thread Pool Governor: Adapt worker count to queue depth
        if self.telemetry.queued_tasks > self.current_workers
            && self.current_workers < self.max_workers
        {
            let target = (self.current_workers + 2).min(self.max_workers);
            let action = AdaptationAction::ScaleWorkers {
                target_workers: target,
                reason: format!(
                    "Queued tasks ({}) exceed active workers ({})",
                    self.telemetry.queued_tasks, self.current_workers
                ),
            };
            self.history.push(action.clone());
            self.current_workers = target;
            return action;
        } else if self.telemetry.queued_tasks == 0
            && self.telemetry.active_tasks < self.current_workers / 2
            && self.current_workers > self.min_workers
        {
            let target = (self.current_workers / 2).max(self.min_workers);
            let action = AdaptationAction::ScaleWorkers {
                target_workers: target,
                reason: "Idle worker pool reduction (idle load detected)".to_string(),
            };
            self.history.push(action.clone());
            self.current_workers = target;
            return action;
        }

        AdaptationAction::Maintain
    }

    /// Format summary of runtime adaptation decisions
    pub fn report_summary(&self) -> String {
        let mut s = String::new();
        s.push_str("=== Syntra Self-Adapting Runtime (SAR) Report ===\n");
        s.push_str(&format!(
            "  Status:           {}\n",
            if self.enabled {
                "Active (governed)"
            } else {
                "Disabled"
            }
        ));
        s.push_str(&format!(
            "  Worker Pool:      {} workers (min: {}, max: {})\n",
            self.current_workers, self.min_workers, self.max_workers
        ));
        s.push_str(&format!(
            "  Heap Allocated:   {} KB\n",
            self.telemetry.allocated_bytes / 1024
        ));
        s.push_str(&format!(
            "  Recycle Pacing:   Every {} ops\n",
            self.recycle_pacing_ops
        ));
        s.push_str(&format!(
            "  Code Cache:       {} entries ({} bytes)\n",
            self.code_cache.len(),
            self.code_cache.current_memory()
        ));
        s.push_str(&format!(
            "  Deopt Events:     {}\n",
            self.deopt_manager.events().len()
        ));
        s.push_str(&format!("  Adaptation Steps: {}\n", self.history.len()));
        for (i, action) in self.history.iter().enumerate() {
            s.push_str(&format!("    [{}] {:?}\n", i + 1, action));
        }
        s
    }
}

pub type SharedSarEngine = Arc<Mutex<SarEngine>>;

pub fn create_sar_engine() -> (SarEngine, SharedSarEngine) {
    let engine = SarEngine::new_active();
    let shared = Arc::new(Mutex::new(SarEngine::new_active()));
    (engine, shared)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sar_disabled_by_default_run020() {
        let mut sar = SarEngine::new();
        assert!(
            !sar.is_enabled(),
            "RUN-020: Adaptation MUST be disabled by default"
        );

        sar.record_task_queue(1, 100);
        let action = sar.evaluate();
        assert_eq!(
            action,
            AdaptationAction::Maintain,
            "Disabled SAR engine must not adapt"
        );
    }

    #[test]
    fn test_sar_memory_pressure_adaptation() {
        let mut sar = SarEngine::new_active();
        sar.telemetry.allocation_limit_bytes = 1_000_000;
        sar.record_allocation(900_000); // 90% allocation

        let action = sar.evaluate();
        match action {
            AdaptationAction::AdjustMemoryPacing {
                recycle_frequency_ops,
                reason,
            } => {
                assert_eq!(recycle_frequency_ops, 1_000);
                assert!(reason.contains("High memory watermark"));
            }
            other => panic!("Expected AdjustMemoryPacing, got {other:?}"),
        }
    }

    #[test]
    fn test_sar_worker_scale_up_and_down() {
        let mut sar = SarEngine::new_active();
        sar.current_workers = 4;
        sar.max_workers = 16;
        sar.min_workers = 2;

        // Queue heavy load
        sar.record_task_queue(4, 10);
        let action = sar.evaluate();
        assert!(matches!(
            action,
            AdaptationAction::ScaleWorkers {
                target_workers: 6,
                ..
            }
        ));

        // Idle load
        sar.record_task_queue(1, 0);
        let action2 = sar.evaluate();
        assert!(matches!(
            action2,
            AdaptationAction::ScaleWorkers {
                target_workers: 3,
                ..
            }
        ));
    }

    #[test]
    fn test_sar_background_compilation_and_guard_deopt_loop() {
        let mut sar = SarEngine::new_active();

        // 1. Background compilation request
        sar.request_specialisation(SpecialisationRequest {
            function_name: "calculate_tax".to_string(),
            strategy_id: "VectorizedTax".to_string(),
            air_digest: "air_hash_tax".to_string(),
            estimated_speedup_pct: 25.0,
            guards: vec![
                DynamicGuard::RangeGuard {
                    param_idx: 0,
                    min_val: 0,
                    max_val: 50_000,
                },
                DynamicGuard::MonomorphicReceiverGuard {
                    expected_class: "CorporateTax".to_string(),
                },
            ],
        });

        // 2. Poll and activate
        let action = sar.evaluate();
        assert!(matches!(
            action,
            AdaptationAction::ActivateSpecialisation { .. }
        ));
        assert_eq!(sar.code_cache.len(), 1);

        // 3. In-bounds call passes guards
        let pass_res = sar.verify_call_guards(
            "calculate_tax",
            "VectorizedTax",
            Some("CorporateTax"),
            &[Value::Int(10_000)],
            0,
        );
        assert!(pass_res.is_ok());

        // 4. Out-of-bounds call violates range guard -> triggers safe deopt
        let fail_res = sar.verify_call_guards(
            "calculate_tax",
            "VectorizedTax",
            Some("CorporateTax"),
            &[Value::Int(999_999)],
            4,
        );
        assert!(fail_res.is_err());
        assert_eq!(sar.deopt_manager.events().len(), 1);

        // 5. Fault-injection test (Phase 8 Gate)
        sar.inject_fault("calculate_tax", "VectorizedTax");
        sar.inject_fault("calculate_tax", "VectorizedTax"); // Reaches threshold 3

        assert!(sar.deopt_manager.is_blacklisted("calculate_tax"));
        assert!(sar
            .history
            .iter()
            .any(|a| matches!(a, AdaptationAction::RetireSpecialisation { .. })));
    }
}
