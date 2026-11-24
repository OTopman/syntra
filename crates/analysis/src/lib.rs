//! # Syntra Whole-Program Analysis Subsystem
//!
//! Provides static call-graph construction, escape analysis, side-effect lattice computation,
//! points-to and alias analysis, and fine-grained incremental cache invalidation.

pub mod alias;
pub mod callgraph;
pub mod effects;
pub mod escape;
pub mod invalidation;

pub use alias::{AliasAnalyzer, AliasReport, AliasResult};
pub use callgraph::{CallGraph, CallNode};
pub use effects::{EffectAnalyzer, EffectReport, EffectSet};
pub use escape::{
    AllocationFact, EscapeAnalyzer, EscapeReason, EscapeReport, EscapeState, ProofStatus,
};
pub use invalidation::{AnalysisCache, CachedFunctionFacts, InvalidationReport};

use syntra_air::ir::AirModule;

/// Comprehensive analysis report capturing whole-program static and dynamic properties.
#[derive(Debug, Clone)]
pub struct AnalysisReport {
    /// Whole-program call graph and invocation hierarchy.
    pub call_graph: CallGraph,
    /// Object escape analysis proving heap allocation vs stack promotion facts.
    pub escape_report: EscapeReport,
    /// Function side-effect summaries (purity, I/O, mutation).
    pub effect_report: EffectReport,
    /// Pointer alias and disjoint memory region analysis.
    pub alias_report: AliasReport,
    /// Cache dependency mappings for incremental compilation invalidation.
    pub invalidation_report: InvalidationReport,
    /// Optional dynamic execution telemetry from PGO profile runs.
    pub profile_data: Option<syntra_runtime::RuntimeProfileData>,
}

impl AnalysisReport {
    pub fn print_summary(&self) -> String {
        let mut out = String::new();
        out.push_str("========================================\n");
        out.push_str(" Syntra Whole-Program Analysis Report\n");
        out.push_str("========================================\n");
        out.push_str(&format!(
            "• Functions Analyzed: {}\n",
            self.call_graph.nodes.len()
        ));
        out.push_str(&format!(
            "• Entry Points: {:?}\n",
            self.call_graph.entry_points
        ));

        // Side-Effect Summaries
        out.push_str("\n--- Function Side-Effect Lattice (Phase 4) ---\n");
        out.push_str(&format!(
            "  • Pure (Deterministic): {} function(s)\n",
            self.effect_report.pure_functions.len()
        ));
        if !self.effect_report.pure_functions.is_empty() {
            out.push_str(&format!("    -> {:?}\n", self.effect_report.pure_functions));
        }
        out.push_str(&format!(
            "  • I/O Operations:       {} function(s)\n",
            self.effect_report.io_functions.len()
        ));
        if !self.effect_report.io_functions.is_empty() {
            out.push_str(&format!("    -> {:?}\n", self.effect_report.io_functions));
        }
        out.push_str(&format!(
            "  • Concurrency/Channels: {} function(s)\n",
            self.effect_report.concurrent_functions.len()
        ));
        if !self.effect_report.concurrent_functions.is_empty() {
            out.push_str(&format!(
                "    -> {:?}\n",
                self.effect_report.concurrent_functions
            ));
        }

        // Alias Analysis
        out.push_str("\n--- Points-To & Alias Disambiguation ---\n");
        out.push_str(&format!(
            "  • Disjoint Pointer Pairs (NoAlias):   {}\n",
            self.alias_report.proven_no_alias
        ));
        out.push_str(&format!(
            "  • Identical Pointer Pairs (MustAlias): {}\n",
            self.alias_report.must_alias
        ));
        out.push_str(&format!(
            "  • Conservative Fallbacks (MayAlias):  {}\n",
            self.alias_report.may_alias
        ));

        // Incremental Invalidation
        out.push_str("\n--- Incremental Fact Invalidation Engine ---\n");
        out.push_str(&format!(
            "  • Cached Function Facts: {}\n",
            self.invalidation_report.cache_hits
        ));
        out.push_str(&format!(
            "  • Recomputed / Invalidated: {}\n",
            self.invalidation_report.invalidated_and_recomputed
        ));

        // Escape & Memory Invariants
        out.push_str("\n--- Escape & Memory Domain Analysis ---\n");
        out.push_str(&format!(
            "  • Total Heap/Arena Allocations: {}\n",
            self.escape_report.total_allocations
        ));
        out.push_str(&format!(
            "  • Proven Non-Escaping (Stack-Promotable): {}\n",
            self.escape_report.proven_non_escaping
        ));
        out.push_str(&format!(
            "  • Escaped Allocations: {}\n",
            self.escape_report.escaped_allocations
        ));
        if self.escape_report.arena_violations > 0 {
            out.push_str(&format!(
                "  • ⚠️  [MEM-023] Arena Domain Invariant Violations: {}\n",
                self.escape_report.arena_violations
            ));
        } else {
            out.push_str("  • [MEM-023] Arena Domain Invariants: All arena references safely contained within region lifetime.\n");
        }

        // PGO Feedback
        if let Some(profile) = &self.profile_data {
            out.push_str("\n--- Profile-Guided Optimization (PGO) Feedback ---\n");
            out.push_str(&format!(
                "  • Profile Source: Dynamic execution telemetry ({} calls recorded)\n",
                profile.total_calls
            ));
            out.push_str(&format!(
                "  • Platform Target: {} ({})\n",
                profile.hardware.os, profile.hardware.arch
            ));
            for (func, fp) in &profile.functions {
                let hot = if profile.is_hot(func) {
                    " [🔥 HOT CALL TARGET - INLINE PRIORITY]"
                } else {
                    ""
                };
                out.push_str(&format!(
                    "  • {}(): {} calls, {:.1} µs, {} allocs{}\n",
                    func,
                    fp.call_count,
                    fp.total_duration.as_micros() as f64,
                    fp.allocation_count,
                    hot
                ));
            }
        }

        // Allocation Proof Registry
        if !self.escape_report.facts.is_empty() {
            out.push_str("\n--- Allocation Proof Registry ---\n");
            for fact in &self.escape_report.facts {
                let status_str = match fact.status {
                    escape::ProofStatus::Proven => "PROVEN",
                    escape::ProofStatus::Checked => "CHECKED",
                    escape::ProofStatus::Observed => "OBSERVED",
                    escape::ProofStatus::Predicted => "PREDICTED",
                };
                let escape_str = match &fact.escape_state {
                    escape::EscapeState::NoEscape => {
                        "NON-ESCAPING -> Promoted to Stack".to_string()
                    }
                    escape::EscapeState::Escaped(reason) => format!("ESCAPED ({reason:?})"),
                };
                out.push_str(&format!(
                    "  [{}] {}: {} in {}() => {}\n",
                    status_str, fact.value_id, fact.ty, fact.function_name, escape_str
                ));
            }
        }
        out.push_str("========================================\n");
        out
    }
}

pub fn analyze_module(module: &AirModule) -> AnalysisReport {
    analyze_module_with_profile(module, None)
}

pub fn analyze_module_with_profile(
    module: &AirModule,
    profile_data: Option<syntra_runtime::RuntimeProfileData>,
) -> AnalysisReport {
    let call_graph = CallGraph::build(module);
    let escape_report = EscapeAnalyzer::analyze_module(module);
    let effect_report = EffectAnalyzer::analyze_module(module, &call_graph);
    let alias_report = AliasAnalyzer::analyze_module(module);

    let mut cache = AnalysisCache::new();
    let invalidation_report = cache.update(module, &call_graph);

    AnalysisReport {
        call_graph,
        escape_report,
        effect_report,
        alias_report,
        invalidation_report,
        profile_data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syntra_air::lower_and_verify;

    #[test]
    fn test_escape_analysis_and_callgraph() {
        let code = r#"
class Point {
    public int x;
    public int y;
}

func compute(): int {
    Point p = new Point();
    p.x = 10;
    return p.x;
}

func main(): int {
    return compute();
}
"#;
        let prog = syntra_syntax::parse(code).expect("Parse failed");
        let module = lower_and_verify(&prog).expect("AIR failed");
        let report = analyze_module(&module);

        assert!(report.call_graph.nodes.contains_key("main"));
        assert!(report.call_graph.nodes.contains_key("compute"));
        assert_eq!(report.escape_report.proven_non_escaping, 1);
        assert!(!report.effect_report.get_effects("compute").io);
    }

    #[test]
    fn test_effect_analysis_io_detection() {
        let code = r#"
func log_msg() {
    println("Log event");
}

func pure_add(int a, int b): int {
    return a + b;
}

func main(): int {
    log_msg();
    return pure_add(10, 20);
}
"#;
        let prog = syntra_syntax::parse(code).expect("Parse failed");
        let module = lower_and_verify(&prog).expect("AIR failed");
        let report = analyze_module(&module);

        assert!(report.effect_report.is_function_pure("pure_add"));
        assert!(!report.effect_report.is_function_pure("log_msg"));
        assert!(report.effect_report.get_effects("log_msg").io);
        // main calls log_msg so main transitively has IO effect
        assert!(report.effect_report.get_effects("main").io);
    }

    #[test]
    fn test_incremental_invalidation_cache() {
        let code = r#"
func add(int a, int b): int { return a + b; }
func main(): int { return add(1, 2); }
"#;
        let prog = syntra_syntax::parse(code).expect("Parse failed");
        let module = lower_and_verify(&prog).expect("AIR failed");
        let call_graph = CallGraph::build(&module);

        let mut cache = AnalysisCache::new();
        let inv1 = cache.update(&module, &call_graph);
        assert_eq!(inv1.invalidated_and_recomputed, 2);
        assert_eq!(inv1.cache_hits, 0);

        // Second update with identical module: all entries hit the cache!
        let inv2 = cache.update(&module, &call_graph);
        assert_eq!(inv2.invalidated_and_recomputed, 0);
        assert_eq!(inv2.cache_hits, 2);
    }
}
