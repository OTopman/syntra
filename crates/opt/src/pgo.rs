use crate::OptimizationPass;
use std::collections::{HashMap, HashSet};
use syntra_air::ir::{AirModule, Op};
use syntra_diagnostics::Diagnostic;

/// Profile-Guided Optimization (PGO) pass (Track C / Subsystem 9)
/// Consumes deterministic profile lockfile metadata (profile.synlock) to drive:
/// 1. Hot function prioritization
/// 2. Inlining of small leaf functions called inside hot functions
/// 3. Invariant check preservation
#[derive(Debug, Clone)]
pub struct PgoOptimizationPass {
    pub hot_functions: HashSet<String>,
    pub function_calls: HashMap<String, u64>,
}

impl PgoOptimizationPass {
    pub fn new(hot_functions: HashSet<String>, function_calls: HashMap<String, u64>) -> Self {
        Self {
            hot_functions,
            function_calls,
        }
    }

    pub fn is_hot(&self, name: &str) -> bool {
        self.hot_functions.contains(name)
    }

    /// Run PGO pass across all functions in the module
    pub fn run_on_module(&self, module: &mut AirModule) -> usize {
        let mut inlined_count = 0;

        // Collect small leaf functions (1 basic block, returns constant or binary op, no complex calls)
        let mut leaf_candidates = HashMap::new();
        for func in &module.functions {
            if func.blocks.len() == 1 {
                let bb = &func.blocks[0];
                if bb.instructions.len() <= 6 {
                    let is_leaf = bb
                        .instructions
                        .iter()
                        .all(|inst| !matches!(inst.op, Op::Call { .. } | Op::NewObject { .. }));
                    if is_leaf {
                        leaf_candidates.insert(func.name.clone(), func.clone());
                    }
                }
            }
        }

        // For each hot function, optimize call sites to leaf candidates where applicable
        for func in &mut module.functions {
            if self.is_hot(&func.name) {
                for bb in &mut func.blocks {
                    for inst in &mut bb.instructions {
                        if let Op::Call { func_name, args } = &inst.op {
                            if let Some(leaf) = leaf_candidates.get(func_name) {
                                // If leaf is a simple single-instruction computation (e.g. add(a, b) -> a + b)
                                if leaf.blocks.len() == 1 && leaf.blocks[0].instructions.len() == 1
                                {
                                    let leaf_inst = &leaf.blocks[0].instructions[0];
                                    if let Op::Binary { op, .. } = &leaf_inst.op {
                                        if args.len() >= 2 {
                                            inst.op = Op::Binary {
                                                op: *op,
                                                left: args[0],
                                                right: args[1],
                                            };
                                            inlined_count += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        inlined_count
    }
}

impl OptimizationPass for PgoOptimizationPass {
    fn name(&self) -> &'static str {
        "ProfileGuidedOptimization"
    }

    fn run(&self, module: &mut AirModule) -> Result<bool, Vec<Diagnostic>> {
        let count = self.run_on_module(module);
        Ok(count > 0)
    }
}
