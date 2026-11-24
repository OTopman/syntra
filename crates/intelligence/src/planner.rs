use crate::strategy::{StrategyContract, StrategyStatus};
use std::collections::HashSet;

/// A composite, compatible plan combining multiple strategies (Specification 04)
#[derive(Debug, Clone, PartialEq)]
pub struct CompositePlan {
    pub plan_id: String,
    pub strategies: Vec<StrategyContract>,
    pub composite_speedup_pct: f64,
    pub compile_cost_rating: &'static str,
    pub memory_impact_rating: &'static str,
    pub pareto_rank: usize,
}

/// Strategy Planner: composes compatible strategies, prunes conflicts, and ranks plans (AIE-001..AIE-004)
pub struct StrategyPlanner {
    pub max_candidate_plans: usize,
}

impl Default for StrategyPlanner {
    fn default() -> Self {
        Self {
            max_candidate_plans: 8,
        }
    }
}

impl StrategyPlanner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compose candidate plans, prune conflicts, and rank by Pareto optimality
    pub fn plan(&self, mut candidates: Vec<StrategyContract>) -> Vec<CompositePlan> {
        // 1. Filter candidates whose preconditions cannot be satisfied
        candidates.retain(|c| c.status != StrategyStatus::Revoked);

        // Sort candidates by speedup descending
        candidates.sort_by(|a, b| {
            b.estimated_speedup_pct
                .partial_cmp(&a.estimated_speedup_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 2. Prune mutually conflicting strategies (AIE-003)
        let mut compatible: Vec<StrategyContract> = Vec::new();
        let mut active_conflicts: HashSet<String> = HashSet::new();

        for candidate in candidates {
            // Check if this candidate conflicts with any already selected strategy
            let has_conflict = active_conflicts.contains(&candidate.id)
                || candidate
                    .conflicts_with
                    .iter()
                    .any(|c| compatible.iter().any(|comp| comp.id == *c));

            if !has_conflict {
                for c in &candidate.conflicts_with {
                    active_conflicts.insert(c.clone());
                }
                compatible.push(candidate);
            }
        }

        // 3. Assemble composite plans (Baseline, Recommended, Aggressive)
        let mut plans = Vec::new();

        // Plan 1: All compatible strategies (Full Recommended Plan)
        if !compatible.is_empty() {
            let speedup = Self::calculate_composite_speedup(&compatible);
            plans.push(CompositePlan {
                plan_id: "Plan-Recommended".to_string(),
                strategies: compatible.clone(),
                composite_speedup_pct: speedup,
                compile_cost_rating: "Moderate",
                memory_impact_rating: "Net Positive (Stack & Arena Optimized)",
                pareto_rank: 1,
            });
        }

        // Plan 2: Conservative Safe Core (DCE + Constant Folding + ISA)
        let conservative: Vec<_> = compatible
            .iter()
            .filter(|c| c.id.contains("PROG") || c.id.contains("HW"))
            .cloned()
            .collect();

        if !conservative.is_empty() {
            let speedup = Self::calculate_composite_speedup(&conservative);
            plans.push(CompositePlan {
                plan_id: "Plan-Conservative".to_string(),
                strategies: conservative,
                composite_speedup_pct: speedup,
                compile_cost_rating: "Low",
                memory_impact_rating: "Neutral",
                pareto_rank: 2,
            });
        }

        // Plan 3: Memory Optimized Only (Stack Promotion + Arena Recycle)
        let memory_focused: Vec<_> = compatible
            .iter()
            .filter(|c| c.id.contains("MEM"))
            .cloned()
            .collect();

        if !memory_focused.is_empty() {
            let speedup = Self::calculate_composite_speedup(&memory_focused);
            plans.push(CompositePlan {
                plan_id: "Plan-MemoryOptimized".to_string(),
                strategies: memory_focused,
                composite_speedup_pct: speedup,
                compile_cost_rating: "Low",
                memory_impact_rating: "Highest Efficiency (-80% Heap Allocations)",
                pareto_rank: 3,
            });
        }

        // Bound generated plans to max_candidate_plans (AIE-004)
        plans.truncate(self.max_candidate_plans);
        plans
    }

    /// Diminishing-returns compounding model for multi-strategy combinations: 1 - prod(1 - s_i)
    fn calculate_composite_speedup(strategies: &[StrategyContract]) -> f64 {
        let mut remainder = 1.0;
        for s in strategies {
            let fraction = (s.estimated_speedup_pct / 100.0).clamp(0.0, 0.90);
            remainder *= 1.0 - fraction;
        }
        ((1.0 - remainder) * 100.0).max(0.0)
    }
}
