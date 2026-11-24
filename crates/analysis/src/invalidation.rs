use crate::callgraph::CallGraph;
use crate::effects::{EffectAnalyzer, EffectSet};
use crate::escape::{AllocationFact, EscapeAnalyzer};
use std::collections::{HashMap, HashSet};
use syntra_air::ir::{AirFunction, AirModule};

/// Cached facts for an individual AIR function
#[derive(Debug, Clone)]
pub struct CachedFunctionFacts {
    pub function_name: String,
    pub content_hash: u64,
    pub escape_facts: Vec<AllocationFact>,
    pub effects: EffectSet,
    pub callees: HashSet<String>,
}

/// Incremental Invalidation Report (Specification Phase 4)
#[derive(Debug, Clone, Default)]
pub struct InvalidationReport {
    pub total_functions: usize,
    pub cache_hits: usize,
    pub invalidated_and_recomputed: usize,
    pub invalidated_functions: Vec<String>,
}

/// AnalysisCache maintains incrementally invalidated program facts
#[derive(Debug, Clone, Default)]
pub struct AnalysisCache {
    pub entries: HashMap<String, CachedFunctionFacts>,
}

impl AnalysisCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Computes a structural content hash for an AIR function
    pub fn compute_function_hash(func: &AirFunction) -> u64 {
        let mut hash: u64 = 5381;
        for b in func.name.as_bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(*b as u64);
        }
        hash = hash.wrapping_mul(33).wrapping_add(func.blocks.len() as u64);
        for block in &func.blocks {
            hash = hash
                .wrapping_mul(33)
                .wrapping_add(block.instructions.len() as u64);
            for inst in &block.instructions {
                hash = hash.wrapping_mul(33).wrapping_add(inst.result.0 as u64);
            }
        }
        hash
    }

    /// Incrementally updates facts, invalidating only modified functions and their callers
    pub fn update(&mut self, module: &AirModule, call_graph: &CallGraph) -> InvalidationReport {
        let mut report = InvalidationReport {
            total_functions: module.functions.len(),
            ..Default::default()
        };

        // 1. Identify directly modified or newly introduced functions
        let mut directly_modified = HashSet::new();
        let mut current_hashes = HashMap::new();

        for func in &module.functions {
            let h = Self::compute_function_hash(func);
            current_hashes.insert(func.name.clone(), h);
            match self.entries.get(&func.name) {
                Some(cached) if cached.content_hash == h => {}
                _ => {
                    directly_modified.insert(func.name.clone());
                }
            }
        }

        // 2. Transitive invalidation: Invalidate all callers of modified functions
        let mut to_invalidate = HashSet::new();
        let mut queue: Vec<String> = directly_modified.into_iter().collect();

        while let Some(func_name) = queue.pop() {
            if to_invalidate.insert(func_name.clone()) {
                if let Some(node) = call_graph.nodes.get(&func_name) {
                    for caller in &node.callers {
                        if !to_invalidate.contains(caller) {
                            queue.push(caller.clone());
                        }
                    }
                }
            }
        }

        // 3. Purge invalidated entries from cache
        for name in &to_invalidate {
            self.entries.remove(name);
            report.invalidated_functions.push(name.clone());
        }
        report.invalidated_functions.sort();
        report.invalidated_and_recomputed = to_invalidate.len();
        report.cache_hits = module.functions.len().saturating_sub(to_invalidate.len());

        // 4. Recompute facts for invalidated functions
        for func in &module.functions {
            if to_invalidate.contains(&func.name) {
                let escape_facts = EscapeAnalyzer::analyze_function(func, module);
                let effects = EffectAnalyzer::analyze_module(module, call_graph)
                    .function_effects
                    .get(&func.name)
                    .copied()
                    .unwrap_or_default();

                let callees = call_graph
                    .nodes
                    .get(&func.name)
                    .map(|n| n.callees.clone())
                    .unwrap_or_default();

                let hash = current_hashes.get(&func.name).copied().unwrap_or(0);

                self.entries.insert(
                    func.name.clone(),
                    CachedFunctionFacts {
                        function_name: func.name.clone(),
                        content_hash: hash,
                        escape_facts,
                        effects,
                        callees,
                    },
                );
            }
        }

        report
    }
}
