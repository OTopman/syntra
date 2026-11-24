use crate::callgraph::CallGraph;
use std::collections::{HashMap, HashSet};
use syntra_air::ir::{AirFunction, AirModule, MemoryDomain, Op};

/// Distinct observable side-effects of an AIR function
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EffectSet {
    /// Reads from heap memory or object fields
    pub reads_memory: bool,
    /// Mutates heap memory or object fields
    pub writes_memory: bool,
    /// Allocates heap or arena memory
    pub allocates: bool,
    /// Performs console, filesystem, network, or OS I/O
    pub io: bool,
    /// Spawns worker threads, sends/receives over channels
    pub concurrency: bool,
    /// Contains speculative guards that may trigger deoptimisation
    pub can_deopt: bool,
}

impl EffectSet {
    pub const PURE: Self = Self {
        reads_memory: false,
        writes_memory: false,
        allocates: false,
        io: false,
        concurrency: false,
        can_deopt: false,
    };

    pub fn is_pure(&self) -> bool {
        !self.reads_memory
            && !self.writes_memory
            && !self.io
            && !self.concurrency
            && !self.can_deopt
    }

    pub fn is_read_only(&self) -> bool {
        !self.writes_memory && !self.io && !self.concurrency
    }

    pub fn union(&self, other: &Self) -> Self {
        Self {
            reads_memory: self.reads_memory || other.reads_memory,
            writes_memory: self.writes_memory || other.writes_memory,
            allocates: self.allocates || other.allocates,
            io: self.io || other.io,
            concurrency: self.concurrency || other.concurrency,
            can_deopt: self.can_deopt || other.can_deopt,
        }
    }

    pub fn summary_tag(&self) -> String {
        if self.is_pure() {
            if self.allocates {
                "PURE (Allocating)".to_string()
            } else {
                "PURE (Deterministic)".to_string()
            }
        } else {
            let mut parts = Vec::new();
            if self.io {
                parts.push("IO");
            }
            if self.concurrency {
                parts.push("CONCURRENCY");
            }
            if self.writes_memory {
                parts.push("MUTATES");
            } else if self.reads_memory {
                parts.push("READS");
            }
            if self.allocates {
                parts.push("ALLOC");
            }
            if self.can_deopt {
                parts.push("DEOPT-GUARDED");
            }
            parts.join(" | ")
        }
    }
}

/// Interprocedural side-effect analysis engine
#[derive(Debug, Clone, Default)]
pub struct EffectReport {
    pub function_effects: HashMap<String, EffectSet>,
    pub pure_functions: Vec<String>,
    pub io_functions: Vec<String>,
    pub concurrent_functions: Vec<String>,
}

impl EffectReport {
    pub fn is_function_pure(&self, name: &str) -> bool {
        self.function_effects
            .get(name)
            .map(|e| e.is_pure())
            .unwrap_or(false)
    }

    pub fn get_effects(&self, name: &str) -> EffectSet {
        self.function_effects.get(name).copied().unwrap_or_default()
    }
}

pub struct EffectAnalyzer;

impl EffectAnalyzer {
    pub fn analyze_module(module: &AirModule, call_graph: &CallGraph) -> EffectReport {
        let mut effects: HashMap<String, EffectSet> = HashMap::new();

        // 1. Initial base effects per function from local instructions
        for func in &module.functions {
            effects.insert(func.name.clone(), Self::analyze_function_base(func));
        }

        // 2. Fixed-point interprocedural propagation along call graph edges
        let mut changed = true;
        let mut iterations = 0;
        let max_iterations = module.functions.len() * 2 + 10;

        while changed && iterations < max_iterations {
            changed = false;
            iterations += 1;

            for func in &module.functions {
                let current_effect = effects.get(&func.name).copied().unwrap_or_default();
                let mut combined = current_effect;

                if let Some(node) = call_graph.nodes.get(&func.name) {
                    for callee in &node.callees {
                        if let Some(callee_effect) = effects.get(callee) {
                            combined = combined.union(callee_effect);
                        } else {
                            // Foreign or runtime intrinsic function
                            combined = combined.union(&Self::intrinsic_effect(callee));
                        }
                    }
                }

                if combined != current_effect {
                    effects.insert(func.name.clone(), combined);
                    changed = true;
                }
            }
        }

        let mut pure_functions = Vec::new();
        let mut io_functions = Vec::new();
        let mut concurrent_functions = Vec::new();

        for (name, eff) in &effects {
            if eff.is_pure() {
                pure_functions.push(name.clone());
            }
            if eff.io {
                io_functions.push(name.clone());
            }
            if eff.concurrency {
                concurrent_functions.push(name.clone());
            }
        }

        pure_functions.sort();
        io_functions.sort();
        concurrent_functions.sort();

        EffectReport {
            function_effects: effects,
            pure_functions,
            io_functions,
            concurrent_functions,
        }
    }

    fn analyze_function_base(func: &AirFunction) -> EffectSet {
        let mut eff = EffectSet::default();
        let mut stack_slots = HashSet::new();

        // 1. Identify local stack slots
        for block in &func.blocks {
            for inst in &block.instructions {
                if let Op::Alloc {
                    domain: MemoryDomain::Stack,
                    ..
                } = &inst.op
                {
                    stack_slots.insert(inst.result);
                }
            }
        }

        // 2. Classify instructions
        for block in &func.blocks {
            for inst in &block.instructions {
                match &inst.op {
                    Op::Alloc { domain, .. } => {
                        if *domain != MemoryDomain::Stack {
                            eff.allocates = true;
                        }
                    }
                    Op::AllocArray { .. } | Op::NewObject { .. } => {
                        eff.allocates = true;
                    }
                    Op::Load { ptr } => {
                        if !stack_slots.contains(ptr) {
                            eff.reads_memory = true;
                        }
                    }
                    Op::Store { ptr, .. } => {
                        if !stack_slots.contains(ptr) {
                            eff.writes_memory = true;
                        }
                    }
                    Op::GetField { .. } | Op::GetIndex { .. } | Op::ArrayLength { .. } => {
                        eff.reads_memory = true;
                    }
                    Op::SetField { .. } | Op::SetIndex { .. } => {
                        eff.writes_memory = true;
                    }
                    Op::Guard { .. } => {
                        eff.can_deopt = true;
                    }
                    Op::Call { func_name, .. } => {
                        let intrinsic = Self::intrinsic_effect(func_name);
                        eff = eff.union(&intrinsic);
                    }
                    _ => {}
                }
            }
        }

        eff
    }

    fn intrinsic_effect(name: &str) -> EffectSet {
        let mut eff = EffectSet::default();
        if name.contains("print")
            || name.contains("file_")
            || name.contains("http_")
            || name.contains("net_")
            || name.contains("read_line")
        {
            eff.io = true;
            eff.reads_memory = true;
            eff.writes_memory = true;
        } else if name.contains("threadpool")
            || name.contains("channel")
            || name.contains("spawn")
            || name.contains("mutex")
        {
            eff.concurrency = true;
            eff.reads_memory = true;
            eff.writes_memory = true;
        } else if name.contains("alloc") {
            eff.allocates = true;
        }
        eff
    }
}
