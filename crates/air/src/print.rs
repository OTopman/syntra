use crate::ir::*;

pub fn print_air_module(module: &AirModule) -> String {
    let mut out = String::new();
    out.push_str(&format!("; AIR Module: {}\n\n", module.name));

    for func in &module.functions {
        out.push_str(&format!(
            "func @{}({}) -> {} {{\n",
            func.name,
            func.params
                .iter()
                .map(|p| format!("{}: {}", p.id, p.ty))
                .collect::<Vec<_>>()
                .join(", "),
            func.return_type
        ));

        for block in &func.blocks {
            let params_str = if block.params.is_empty() {
                String::new()
            } else {
                format!(
                    "({})",
                    block
                        .params
                        .iter()
                        .map(|(id, ty)| format!("{id}: {ty}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            out.push_str(&format!("  {}:{}\n", block.id, params_str));

            for inst in &block.instructions {
                out.push_str(&format!(
                    "    {} = {}\n",
                    inst.result,
                    format_op(&inst.op, &inst.ty)
                ));
            }

            match &block.terminator {
                Terminator::Return(val, _) => {
                    if let Some(v) = val {
                        out.push_str(&format!("    return {v}\n"));
                    } else {
                        out.push_str("    return\n");
                    }
                }
                Terminator::Branch(target, args, _) => {
                    out.push_str(&format!(
                        "    br {}({})\n",
                        target,
                        args.iter()
                            .map(|a| a.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                Terminator::CondBranch {
                    cond,
                    then_block,
                    then_args,
                    else_block,
                    else_args,
                    ..
                } => {
                    out.push_str(&format!(
                        "    br_cond {}, {}({}), {}({})\n",
                        cond,
                        then_block,
                        then_args
                            .iter()
                            .map(|a| a.to_string())
                            .collect::<Vec<_>>()
                            .join(", "),
                        else_block,
                        else_args
                            .iter()
                            .map(|a| a.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }
        }

        out.push_str("}\n\n");
    }

    out
}

fn format_op(op: &Op, ty: &syntra_frontend::Type) -> String {
    match op {
        Op::ConstInt(i) => format!("iconst.{ty} {i}"),
        Op::ConstFloat(f) => format!("fconst.{ty} {f}"),
        Op::ConstBool(b) => format!("bconst {b}"),
        Op::ConstString(s) => format!("sconst {s:?}"),
        Op::Binary { op, left, right } => format!("{op:?} {left}, {right}"),
        Op::Alloc { domain, ty } => format!("alloc.{domain} {ty}"),
        Op::Load { ptr } => format!("load {ptr}"),
        Op::Store { ptr, val } => format!("store {ptr}, {val}"),
        Op::GetField {
            object,
            field_name,
            offset,
        } => {
            format!("getfield {object}.{field_name} (offset {offset})")
        }
        Op::SetField {
            object,
            field_name,
            offset,
            val,
        } => {
            format!("setfield {object}.{field_name} (offset {offset}), {val}")
        }
        Op::AllocArray { elem_ty, len } => format!("alloc_array.{elem_ty} len {len}"),
        Op::GetIndex { array, index } => format!("getindex {array}[{index}]"),
        Op::SetIndex { array, index, val } => format!("setindex {array}[{index}], {val}"),
        Op::ArrayLength { array } => format!("array_length {array}"),
        Op::Call { func_name, args } => {
            format!(
                "call @{}({})",
                func_name,
                args.iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        Op::FuncAddr { func_name } => format!("func_addr @{func_name}"),
        Op::NewObject {
            class_name,
            domain,
            args,
        } => {
            format!(
                "new.{} @{}({})",
                domain,
                class_name,
                args.iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        Op::Guard {
            cond,
            deopt_target,
            deopt_args,
            reason,
        } => {
            format!(
                "guard {}, deopt -> {}({}) // reason: {}",
                cond,
                deopt_target,
                deopt_args
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                reason
            )
        }
    }
}
