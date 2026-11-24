use std::collections::HashSet;
use syntra_air::ir::{AirFunction, AirModule, MemoryDomain, Op};
use syntra_analysis::escape::EscapeAnalyzer;

pub struct EscapePromotionPass;

impl Default for EscapePromotionPass {
    fn default() -> Self {
        Self::new()
    }
}

impl EscapePromotionPass {
    pub fn new() -> Self {
        Self
    }

    pub fn run_on_module(&self, module: &mut AirModule) -> usize {
        let escape_report = EscapeAnalyzer::analyze_module(module);
        let mut total_promoted = 0;

        for func in &mut module.functions {
            let promotable_ids: HashSet<_> = escape_report
                .facts
                .iter()
                .filter(|f| f.function_name == func.name && f.promotable_to_stack)
                .map(|f| f.value_id)
                .collect();

            if promotable_ids.is_empty() {
                continue;
            }

            total_promoted += Self::run_on_function(func, &promotable_ids);
        }

        total_promoted
    }

    pub fn run_on_function(
        func: &mut AirFunction,
        promotable_ids: &HashSet<syntra_air::ir::ValueId>,
    ) -> usize {
        let mut promoted = 0;

        for block in &mut func.blocks {
            for inst in &mut block.instructions {
                if promotable_ids.contains(&inst.result) {
                    if let Op::Alloc { domain, .. } = &mut inst.op {
                        if *domain != MemoryDomain::Stack {
                            *domain = MemoryDomain::Stack;
                            promoted += 1;
                        }
                    }
                }
            }
        }

        promoted
    }
}
