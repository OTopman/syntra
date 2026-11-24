use std::collections::{HashMap, HashSet};
use syntra_air::ir::*;

pub struct DeadCodeEliminationPass;

impl Default for DeadCodeEliminationPass {
    fn default() -> Self {
        Self::new()
    }
}

impl DeadCodeEliminationPass {
    pub fn new() -> Self {
        Self
    }

    pub fn run_on_function(&self, func: &mut AirFunction) -> bool {
        let mut overall_changed = false;

        loop {
            let mut used: HashSet<ValueId> = HashSet::new();

            // 1. Mark all used values in instructions and terminators
            for block in &func.blocks {
                for inst in &block.instructions {
                    match &inst.op {
                        Op::Binary { left, right, .. } => {
                            used.insert(*left);
                            used.insert(*right);
                        }
                        Op::Store { ptr, val } => {
                            used.insert(*ptr);
                            used.insert(*val);
                        }
                        Op::Load { ptr } => {
                            used.insert(*ptr);
                        }
                        Op::GetField { object, .. } => {
                            used.insert(*object);
                        }
                        Op::SetField { object, val, .. } => {
                            used.insert(*object);
                            used.insert(*val);
                        }
                        Op::AllocArray { len, .. } => {
                            used.insert(*len);
                        }
                        Op::GetIndex { array, index } => {
                            used.insert(*array);
                            used.insert(*index);
                        }
                        Op::SetIndex { array, index, val } => {
                            used.insert(*array);
                            used.insert(*index);
                            used.insert(*val);
                        }
                        Op::ArrayLength { array } => {
                            used.insert(*array);
                        }
                        Op::Call { args, .. } => {
                            for a in args {
                                used.insert(*a);
                            }
                        }
                        Op::NewObject { args, .. } => {
                            for a in args {
                                used.insert(*a);
                            }
                        }
                        Op::Guard {
                            cond, deopt_args, ..
                        } => {
                            used.insert(*cond);
                            for a in deopt_args {
                                used.insert(*a);
                            }
                        }
                        Op::ConstInt(_)
                        | Op::ConstFloat(_)
                        | Op::ConstBool(_)
                        | Op::ConstString(_)
                        | Op::Alloc { .. }
                        | Op::FuncAddr { .. } => {}
                    }
                }

                match &block.terminator {
                    Terminator::Return(val, _) => {
                        if let Some(v) = val {
                            used.insert(*v);
                        }
                    }
                    Terminator::Branch(_, args, _) => {
                        for a in args {
                            used.insert(*a);
                        }
                    }
                    Terminator::CondBranch {
                        cond,
                        then_args,
                        else_args,
                        ..
                    } => {
                        used.insert(*cond);
                        for a in then_args {
                            used.insert(*a);
                        }
                        for a in else_args {
                            used.insert(*a);
                        }
                    }
                }
            }

            // 2. Eliminate unused pure instructions
            let mut pass_changed = false;
            for block in &mut func.blocks {
                let initial_len = block.instructions.len();
                block.instructions.retain(|inst| {
                    match &inst.op {
                        // Instructions with side effects are ALWAYS kept
                        Op::Store { .. }
                        | Op::SetField { .. }
                        | Op::SetIndex { .. }
                        | Op::Call { .. }
                        | Op::NewObject { .. }
                        | Op::Guard { .. } => true,
                        // Pure instructions without side effects can be removed if unused
                        Op::ConstInt(_)
                        | Op::ConstFloat(_)
                        | Op::ConstBool(_)
                        | Op::ConstString(_)
                        | Op::Binary { .. }
                        | Op::Load { .. }
                        | Op::GetField { .. }
                        | Op::GetIndex { .. }
                        | Op::ArrayLength { .. }
                        | Op::FuncAddr { .. } => used.contains(&inst.result),
                        // Keep allocs for now (stack/heap slots)
                        Op::Alloc { .. } | Op::AllocArray { .. } => true,
                    }
                });
                if block.instructions.len() != initial_len {
                    pass_changed = true;
                }
            }

            // 3. Eliminate unreachable blocks
            let mut reachable: HashSet<BlockId> = HashSet::new();
            let mut worklist = vec![func.entry_block];
            reachable.insert(func.entry_block);

            let block_targets: HashMap<BlockId, Vec<BlockId>> = func
                .blocks
                .iter()
                .map(|b| {
                    let targets = match &b.terminator {
                        Terminator::Branch(target, ..) => vec![*target],
                        Terminator::CondBranch {
                            then_block,
                            else_block,
                            ..
                        } => vec![*then_block, *else_block],
                        Terminator::Return(..) => vec![],
                    };
                    (b.id, targets)
                })
                .collect();

            while let Some(bid) = worklist.pop() {
                if let Some(targets) = block_targets.get(&bid) {
                    for t in targets {
                        if reachable.insert(*t) {
                            worklist.push(*t);
                        }
                    }
                }
            }

            let initial_blocks_len = func.blocks.len();
            func.blocks.retain(|b| reachable.contains(&b.id));
            if func.blocks.len() != initial_blocks_len {
                pass_changed = true;
            }

            if pass_changed {
                overall_changed = true;
            } else {
                break;
            }
        }

        overall_changed
    }
}
