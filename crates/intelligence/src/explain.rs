use crate::planner::CompositePlan;
use crate::policy::{PolicyDecision, PolicyMode};
use crate::registry::VersionedModelRegistry;

/// Formats evidence-based explainability summaries for optimization decisions (Specification 04, Section 7)
pub struct ExplainabilityReporter;

impl ExplainabilityReporter {
    pub fn format_report(
        policy_mode: PolicyMode,
        registry: &VersionedModelRegistry,
        plans: &[CompositePlan],
        decision: &PolicyDecision,
    ) -> String {
        let mut out = String::new();
        out.push_str("=========================================================================================\n");
        out.push_str("                       Syntra Intelligence Engine (SIE) Plan                             \n");
        out.push_str("=========================================================================================\n");
        out.push_str(&format!("• Developer Policy: {policy_mode:?}\n"));

        match decision {
            PolicyDecision::Authorized => {
                out.push_str("• Policy Verdict:   ✅ AUTHORIZED for application\n");
            }
            PolicyDecision::RequiresApproval { reason } => {
                out.push_str(&format!(
                    "• Policy Verdict:   ⚠️  REQUIRES DEVELOPER APPROVAL\n  Reason: {reason}\n"
                ));
            }
            PolicyDecision::Rejected { reason } => {
                out.push_str(&format!(
                    "• Policy Verdict:   ❌ REJECTED by Policy Manager\n  Reason: {reason}\n"
                ));
            }
        }

        if let Some(m) = registry.active_model() {
            out.push_str(&format!(
                "• Active ML Model:  {} v{} (Digest: {})\n",
                m.metadata.model_id, m.metadata.version, m.metadata.digest
            ));
            out.push_str(&format!(
                "• Model Evaluation: Accuracy: {:.1}%, Calibration Error: {:.3}\n",
                m.metadata.evaluation_accuracy * 100.0,
                m.metadata.calibration_error
            ));
        }

        out.push_str("-----------------------------------------------------------------------------------------\n");

        if plans.is_empty() {
            out.push_str(
                "No optimization candidates generated (program already at canonical baseline).\n",
            );
        } else {
            for (p_idx, plan) in plans.iter().enumerate() {
                out.push_str(&format!(
                    "Plan [#{}] {} (Pareto Rank: {}, Projected Speedup: +{:.1}%)\n",
                    p_idx + 1,
                    plan.plan_id,
                    plan.pareto_rank,
                    plan.composite_speedup_pct
                ));
                out.push_str(&format!("  Compile Cost:  {}\n", plan.compile_cost_rating));
                out.push_str(&format!("  Memory Impact: {}\n", plan.memory_impact_rating));
                out.push_str("  Composed Strategies:\n");

                for (s_idx, s) in plan.strategies.iter().enumerate() {
                    out.push_str(&format!(
                        "    {}.{} [{}] {} (by {})\n",
                        p_idx + 1,
                        s_idx + 1,
                        s.id,
                        s.name,
                        s.agent_origin
                    ));
                    out.push_str(&format!("        Scope:               {:?}\n", s.scope));
                    out.push_str(&format!(
                        "        Expected Speedup:    +{:.1}% (±{:.1}% uncertainty)\n",
                        s.estimated_speedup_pct, s.uncertainty_margin_pct
                    ));
                    out.push_str(&format!(
                        "        Memory Impact:       {}\n",
                        s.memory_impact
                    ));
                    out.push_str(&format!("        Rationale:           {}\n", s.explanation));
                    if !s.preconditions.is_empty() {
                        out.push_str(&format!(
                            "        Preconditions:       {}\n",
                            s.preconditions.join("; ")
                        ));
                    }
                    if !s.safety_obligations.is_empty() {
                        out.push_str(&format!(
                            "        Safety Obligations:  {}\n",
                            s.safety_obligations.join("; ")
                        ));
                    }
                    if !s.conflicts_with.is_empty() {
                        out.push_str(&format!(
                            "        Resolved Conflicts:  Pruned mutually exclusive {}\n",
                            s.conflicts_with.join(", ")
                        ));
                    }
                }
                out.push('\n');
            }
        }

        out.push_str("=========================================================================================\n");
        out
    }
}
