use std::collections::HashMap;
use syntra_air::ir::*;
use syntra_syntax::ast::BinaryOp;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum ConstVal {
    Int(i64),
    Float(f64),
    Bool(bool),
}

pub struct ConstantFoldingPass;

impl Default for ConstantFoldingPass {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstantFoldingPass {
    pub fn new() -> Self {
        Self
    }

    pub fn run_on_function(&self, func: &mut AirFunction) -> bool {
        let mut changed = false;
        let mut known: HashMap<ValueId, ConstVal> = HashMap::new();

        for block in &mut func.blocks {
            for inst in &mut block.instructions {
                match &inst.op {
                    Op::ConstInt(i) => {
                        known.insert(inst.result, ConstVal::Int(*i));
                    }
                    Op::ConstFloat(f) => {
                        known.insert(inst.result, ConstVal::Float(*f));
                    }
                    Op::ConstBool(b) => {
                        known.insert(inst.result, ConstVal::Bool(*b));
                    }
                    Op::Binary { op, left, right } => {
                        if let (Some(l), Some(r)) = (known.get(left), known.get(right)) {
                            if let (ConstVal::Int(li), ConstVal::Int(ri)) = (l, r) {
                                let folded = match op {
                                    BinaryOp::Add => Some(Op::ConstInt(li.wrapping_add(*ri))),
                                    BinaryOp::Sub => Some(Op::ConstInt(li.wrapping_sub(*ri))),
                                    BinaryOp::Mul => Some(Op::ConstInt(li.wrapping_mul(*ri))),
                                    BinaryOp::Div => {
                                        if *ri != 0 {
                                            Some(Op::ConstInt(li / ri))
                                        } else {
                                            None
                                        }
                                    }
                                    BinaryOp::EqualEqual => Some(Op::ConstBool(li == ri)),
                                    BinaryOp::NotEqual => Some(Op::ConstBool(li != ri)),
                                    BinaryOp::Less => Some(Op::ConstBool(li < ri)),
                                    BinaryOp::LessEqual => Some(Op::ConstBool(li <= ri)),
                                    BinaryOp::Greater => Some(Op::ConstBool(li > ri)),
                                    BinaryOp::GreaterEqual => Some(Op::ConstBool(li >= ri)),
                                    BinaryOp::And | BinaryOp::Or => None,
                                    BinaryOp::Mod => {
                                        if *ri != 0 {
                                            Some(Op::ConstInt(li % ri))
                                        } else {
                                            None
                                        }
                                    }
                                    BinaryOp::BitwiseAnd => Some(Op::ConstInt(li & ri)),
                                    BinaryOp::BitwiseOr => Some(Op::ConstInt(li | ri)),
                                    BinaryOp::BitwiseXor => Some(Op::ConstInt(li ^ ri)),
                                    BinaryOp::Shl => Some(Op::ConstInt(li << (ri & 63))),
                                    BinaryOp::Shr => {
                                        Some(Op::ConstInt(((*li as u64) >> (ri & 63)) as i64))
                                    }
                                };

                                if let Some(new_op) = folded {
                                    match &new_op {
                                        Op::ConstInt(val) => {
                                            known.insert(inst.result, ConstVal::Int(*val));
                                        }
                                        Op::ConstBool(val) => {
                                            known.insert(inst.result, ConstVal::Bool(*val));
                                        }
                                        _ => {}
                                    }
                                    inst.op = new_op;
                                    changed = true;
                                }
                            } else if let (ConstVal::Bool(lb), ConstVal::Bool(rb)) = (l, r) {
                                let folded = match op {
                                    BinaryOp::EqualEqual => Some(Op::ConstBool(lb == rb)),
                                    BinaryOp::NotEqual => Some(Op::ConstBool(lb != rb)),
                                    BinaryOp::And => Some(Op::ConstBool(*lb && *rb)),
                                    BinaryOp::Or => Some(Op::ConstBool(*lb || *rb)),
                                    _ => None,
                                };
                                if let Some(new_op) = folded {
                                    if let Op::ConstBool(val) = new_op {
                                        known.insert(inst.result, ConstVal::Bool(val));
                                    }
                                    inst.op = new_op;
                                    changed = true;
                                }
                            }
                        }
                    }
                    Op::Call { func_name, args } => {
                        if (func_name == "syntra_math_min" || func_name == "min") && args.len() == 2
                        {
                            if let (Some(ConstVal::Int(a)), Some(ConstVal::Int(b))) =
                                (known.get(&args[0]), known.get(&args[1]))
                            {
                                let m = *a.min(b);
                                known.insert(inst.result, ConstVal::Int(m));
                                inst.op = Op::ConstInt(m);
                                changed = true;
                            }
                        } else if (func_name == "syntra_math_max" || func_name == "max")
                            && args.len() == 2
                        {
                            if let (Some(ConstVal::Int(a)), Some(ConstVal::Int(b))) =
                                (known.get(&args[0]), known.get(&args[1]))
                            {
                                let m = *a.max(b);
                                known.insert(inst.result, ConstVal::Int(m));
                                inst.op = Op::ConstInt(m);
                                changed = true;
                            }
                        } else if (func_name == "syntra_math_abs" || func_name == "abs")
                            && args.len() == 1
                        {
                            if let Some(ConstVal::Int(a)) = known.get(&args[0]) {
                                let m = a.abs();
                                known.insert(inst.result, ConstVal::Int(m));
                                inst.op = Op::ConstInt(m);
                                changed = true;
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Fold constant conditional branches
            if let Terminator::CondBranch {
                cond,
                then_block,
                then_args,
                else_block,
                else_args,
                span,
            } = &block.terminator
            {
                if let Some(ConstVal::Bool(b)) = known.get(cond) {
                    if *b {
                        block.terminator =
                            Terminator::Branch(*then_block, then_args.clone(), *span);
                    } else {
                        block.terminator =
                            Terminator::Branch(*else_block, else_args.clone(), *span);
                    }
                    changed = true;
                }
            }
        }

        changed
    }
}
