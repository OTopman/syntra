//! # Syntra Intelligence Engine (SIE) & SLS
//!
//! Autonomous machine-learning driven optimization subsystem conforming to Specification 04 & 08.
//! Features supervised ensemble models, multi-agent cooperative evaluation, rollback safety gates,
//! training data curation with leakage-free splits, and explainability reporting.

pub mod agents;
pub mod dataset;
pub mod evaluation;
pub mod explain;
pub mod governance;
pub mod model;
pub mod planner;
pub mod policy;
pub mod registry;
pub mod strategy;

pub use agents::{
    ExecutionAgent, HardwareAgent, IntelligenceAgent, MemoryAgent, PerformanceAgent,
    ProgrammeAgent, ResearchAgent,
};
pub use dataset::{
    privacy_safe_identifier, FailureReason, OptimizationDataset, ProgramFeatures, TrainingRecord,
};
pub use evaluation::{ModelEvaluationReport, ModelEvaluator, PromotionGateDecision};
pub use explain::ExplainabilityReporter;
pub use governance::{GovernanceAuditEntry, GovernanceAuditLog, QuarantineReason};
pub use model::{
    ExecutionTuningRecommendation, HardwareCostScores, InferenceRequest, InferenceResponse,
    InlineThreshold, MemoryDomainRecommendation, ResourceCostEstimate, StrategyPrediction,
    SupervisedEnsembleModel,
};
pub use planner::{CompositePlan, StrategyPlanner};
pub use policy::{PolicyDecision, PolicyManager, PolicyMode};
pub use registry::{ModelMetadata, ModelPredictor, PredictionResult, VersionedModelRegistry};
pub use strategy::{StrategyContract, StrategyScope, StrategyStatus};

use syntra_air::ir::AirModule;
use syntra_analysis::AnalysisReport;
use syntra_runtime::RuntimeProfileData;

/// The Syntra Intelligence Engine (SIE) Coordinator (Specification 04 & 08)
pub struct IntelligenceCoordinator {
    pub policy_manager: PolicyManager,
    pub planner: StrategyPlanner,
    pub registry: VersionedModelRegistry,
    pub agents: Vec<Box<dyn IntelligenceAgent>>,
}

impl IntelligenceCoordinator {
    pub fn new(mode: PolicyMode) -> Self {
        let mut registry = VersionedModelRegistry::new();
        let dataset_path = std::path::Path::new(".syntra/datasets/training_records.jsonl");
        if dataset_path.is_file() {
            if let Ok(dataset) = OptimizationDataset::load_from_file(dataset_path) {
                if let Some(trained_model) = ModelPredictor::train_from_dataset(&dataset) {
                    registry.register_model(trained_model, None);
                }
            }
        }

        let agents: Vec<Box<dyn IntelligenceAgent>> = vec![
            Box::new(ProgrammeAgent),
            Box::new(MemoryAgent),
            Box::new(ExecutionAgent),
            Box::new(HardwareAgent),
            Box::new(PerformanceAgent),
            Box::new(ResearchAgent),
        ];

        Self {
            policy_manager: PolicyManager::new(mode),
            planner: StrategyPlanner::new(),
            registry,
            agents,
        }
    }

    pub fn model_registry(&self) -> &VersionedModelRegistry {
        &self.registry
    }

    pub fn model_registry_mut(&mut self) -> &mut VersionedModelRegistry {
        &mut self.registry
    }

    /// Full Planning Pipeline (AIE-001): Ingest facts, collect agent proposals, prune conflicts, and evaluate policy
    pub fn plan_full(
        &self,
        module: &AirModule,
        analysis: Option<&AnalysisReport>,
        profile: Option<&RuntimeProfileData>,
    ) -> (Vec<CompositePlan>, PolicyDecision) {
        let features = ProgramFeatures::extract_from_air(module);
        let mut all_proposals = Vec::new();

        // 1. Gather proposals from all 6 specialized domain agents
        for agent in &self.agents {
            let mut agent_props = agent.propose_strategies(module, &features, analysis, profile);
            all_proposals.append(&mut agent_props);
        }

        // 2. Consult Offline Machine-Learned Cost Model (SLS)
        if let Some(model) = self.registry.active_model() {
            let predictions = model.predict_strategies(&features);
            for p in predictions {
                all_proposals.push(
                    StrategyContract::new(
                        &format!("SLS-ML-{}", p.strategy_name),
                        &format!("{} (ML Guided)", p.strategy_name),
                        "LearningSystemAgent",
                        StrategyScope::Module,
                        p.predicted_speedup_pct,
                        "Low (Learned prior)",
                        "Guided by offline Pareto model",
                        &format!(
                            "Offline empirical model '{}' predicted +{:.1}% speedup with {:.0}% confidence based on AST/AIR structural feature vector.",
                            p.model_origin, p.predicted_speedup_pct, p.confidence * 100.0
                        ),
                    )
                    .with_uncertainty(1.0 - p.confidence),
                );
            }
        }

        // 3. Compose compatible plans, prune conflicts, and Pareto rank
        let plans = self.planner.plan(all_proposals);

        // 4. Authorize against developer policy
        let decision = if let Some(top_plan) = plans.first() {
            self.policy_manager.evaluate_plan(top_plan)
        } else {
            PolicyDecision::Authorized
        };

        (plans, decision)
    }

    pub fn format_plan_report(
        &self,
        module: &AirModule,
        analysis: Option<&AnalysisReport>,
        profile: Option<&RuntimeProfileData>,
    ) -> String {
        let (plans, decision) = self.plan_full(module, analysis, profile);
        ExplainabilityReporter::format_report(
            self.policy_manager.mode,
            &self.registry,
            &plans,
            &decision,
        )
    }

    /// Backward-compatible planning method
    pub fn plan_optimizations(&self, module: &AirModule) -> Vec<StrategyContract> {
        let (plans, _) = self.plan_full(module, None, None);
        if let Some(first) = plans.into_iter().next() {
            first.strategies
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinator_agent_and_planner_pipeline() {
        let code = r#"
            func factorial(int n): int {
                if n <= 1 {
                    return 1;
                }
                return n * factorial(n - 1);
            }

            func main(): void {
                int r = factorial(5);
            }
        "#;
        let prog = syntra_syntax::parse(code).expect("Parse failed");
        let module = syntra_air::lower_and_verify(&prog).expect("AIR failed");
        let analysis = syntra_analysis::analyze_module(&module);

        let coordinator = IntelligenceCoordinator::new(PolicyMode::Approve);
        let (plans, decision) = coordinator.plan_full(&module, Some(&analysis), None);

        assert!(!plans.is_empty(), "Should generate candidate plans");
        let top_plan = &plans[0];
        assert!(top_plan.composite_speedup_pct > 0.0);

        // Check that ResearchAgent proposed tail recursion or inlining
        let has_research_or_prog = top_plan
            .strategies
            .iter()
            .any(|s| s.agent_origin == "ResearchAgent" || s.agent_origin == "ProgrammeAgent");
        assert!(has_research_or_prog);

        // Policy mode Approve requires approval
        match decision {
            PolicyDecision::RequiresApproval { .. } => {}
            other => panic!("Expected RequiresApproval under Approve mode, got {other:?}"),
        }
    }

    #[test]
    fn test_planner_conflict_pruning() {
        let s1 = StrategyContract::new(
            "STRAT-A",
            "Inlining",
            "AgentA",
            StrategyScope::Function("foo".to_string()),
            20.0,
            "Low",
            "Neutral",
            "Inline foo",
        )
        .with_conflict("STRAT-B");

        let s2 = StrategyContract::new(
            "STRAT-B",
            "Tail Recursion",
            "AgentB",
            StrategyScope::Function("foo".to_string()),
            15.0,
            "Low",
            "Neutral",
            "Tail recurse foo",
        )
        .with_conflict("STRAT-A");

        let planner = StrategyPlanner::new();
        let plans = planner.plan(vec![s1, s2]);

        assert!(!plans.is_empty());
        let top_strategies = &plans[0].strategies;
        // Conflicting strategy B must have been pruned in favor of higher-speedup strategy A
        assert_eq!(top_strategies.len(), 1);
        assert_eq!(top_strategies[0].id, "STRAT-A");
    }

    #[test]
    fn test_sls_model_evaluation_and_promotion_gate() {
        let baseline = SupervisedEnsembleModel::new_default_baseline("x86_64");
        let mut candidate = baseline.clone();
        candidate.version = "1.1.0-promoted".to_string();

        let mut test_ds = OptimizationDataset::new();
        for i in 0..5 {
            test_ds.append(TrainingRecord {
                feature_schema_version: "1.0.0".to_string(),
                experiment_id: format!("exp_{i}"),
                candidate_digest: format!("cd_{i}"),
                program_digest: format!("prog_{i}"),
                workload_id: format!("w_{i}"),
                features: ProgramFeatures {
                    total_functions: 2,
                    total_instructions: 30,
                    total_allocations: 4,
                    loop_count: 1,
                    call_count: 2,
                },
                target_os: "macos".to_string(),
                target_arch: "x86_64".to_string(),
                candidate_id: format!("c_{i}"),
                strategy_name: "ConstantFolded".to_string(),
                correctness_passed: true,
                compile_time_ms: 20,
                mean_latency_us: 40.0,
                speedup_factor: 1.08,
                binary_size_bytes: 2000,
                sample_count: 5,
                measurement_uncertainty: 0.01,
                is_pareto_optimal: true,
                failure_reason: None,
            });
        }

        let decision = ModelEvaluator::evaluate_for_promotion(&candidate, &baseline, &test_ds);
        match decision {
            PromotionGateDecision::Approved(rep) => {
                assert!(rep.passed_gate);
                assert!(rep.rmse_speedup_pct <= rep.baseline_rmse_speedup_pct * 1.05);
            }
            PromotionGateDecision::Rejected { reason, .. } => {
                panic!("Promotion gate unexpectedly rejected candidate: {reason}");
            }
        }
    }

    #[test]
    fn test_sls_model_rollback_and_revocation() {
        let mut registry = VersionedModelRegistry::new();
        let baseline_version = registry.active_version().to_string();

        let mut new_model = ModelPredictor::new_default_baseline();
        new_model.metadata.version = "2.0.0".to_string();
        new_model.metadata.digest = "digest_v2".to_string();

        registry.register_model(new_model, None);
        assert_eq!(registry.active_version(), "2.0.0");

        // Rollback to baseline
        assert!(registry.rollback_to(&baseline_version, None).is_ok());
        assert_eq!(registry.active_version(), &baseline_version);

        // Revoke version 2.0.0
        assert!(registry
            .revoke("2.0.0", "Security flaw identified in weights", None)
            .is_ok());

        // Attempting to rollback to a revoked model must fail
        assert!(registry.rollback_to("2.0.0", None).is_err());
    }

    #[test]
    fn test_sls_bounded_inference_and_fallback() {
        let model = ModelPredictor::new_default_baseline();

        // 1. Valid in-distribution request
        let normal_req = InferenceRequest {
            feature_schema_version: "1.0.0".to_string(),
            target_hardware: "x86_64".to_string(),
            features: ProgramFeatures {
                total_functions: 3,
                total_instructions: 80,
                total_allocations: 10,
                loop_count: 1,
                call_count: 3,
            },
            candidate_strategies: vec!["EscapePromoted".to_string()],
            max_budget_us: 10_000,
        };
        let resp = model.infer(&normal_req);
        assert!(!resp.is_fallback);
        assert!(!resp.predictions.is_empty());

        // 2. Out-of-distribution (OOD) request triggers conservative fallback
        let ood_req = InferenceRequest {
            feature_schema_version: "1.0.0".to_string(),
            target_hardware: "x86_64".to_string(),
            features: ProgramFeatures {
                total_functions: 100,
                total_instructions: 5_000_000, // Massive OOD instruction count
                total_allocations: 10,
                loop_count: 1,
                call_count: 3,
            },
            candidate_strategies: vec!["EscapePromoted".to_string()],
            max_budget_us: 10_000,
        };
        let ood_resp = model.infer(&ood_req);
        assert!(ood_resp.is_fallback);
        assert_eq!(ood_resp.model_origin, "deterministic-analytical-fallback");
    }
}
