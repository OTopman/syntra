use crate::ir::*;
use aether_diagnostics::{Diagnostic, Span};
use aether_frontend::Type;
use std::collections::HashMap;

pub struct AirVerifier<'a> {
    func: &'a AirFunction,
    defined_values: HashMap<ValueId, Type>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> AirVerifier<'a> {
    pub fn new(func: &'a AirFunction) -> Self {
        Self {
            func,
            defined_values: HashMap::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn verify(mut self) -> Result<(), Vec<Diagnostic>> {
        // AIR-002: Check CFG entry block exists
        let block_map: HashMap<BlockId, &BasicBlock> = self
            .func
            .blocks
            .iter()
            .map(|b| (b.id, b))
            .collect();

        if !block_map.contains_key(&self.func.entry_block) {
            self.diagnostics.push(Diagnostic::error(
                "AIR-002",
                format!("Entry block {} does not exist in function '{}'", self.func.entry_block, self.func.name),
                Span::default(),
            ));
            return Err(self.diagnostics);
        }

        // Register function parameters as defined SSA values
        for p in &self.func.params {
            self.defined_values.insert(p.id, p.ty.clone());
        }

        // Check each block
        for block in &self.func.blocks {
            // Register block parameters
            for (arg_id, arg_ty) in &block.params {
                if self.defined_values.contains_key(arg_id) {
                    self.diagnostics.push(Diagnostic::error(
                        "AIR-001",
                        format!("SSA violation: value {} is defined multiple times", arg_id),
                        Span::default(),
                    ));
                }
                self.defined_values.insert(*arg_id, arg_ty.clone());
            }

            // Check instructions
            for inst in &block.instructions {
                // AIR-001: Unique definition
                if self.defined_values.contains_key(&inst.result) {
                    self.diagnostics.push(Diagnostic::error(
                        "AIR-001",
                        format!("SSA violation: value {} is defined multiple times", inst.result),
                        inst.span,
                    ));
                }

                // Check operand definitions (AIR-001: Dominated/defined before use)
                match &inst.op {
                    Op::Binary { left, right, .. } => {
                        self.check_value_defined(*left, inst.span);
                        self.check_value_defined(*right, inst.span);
                    }
                    Op::Store { ptr, val } => {
                        self.check_value_defined(*ptr, inst.span);
                        self.check_value_defined(*val, inst.span);
                    }
                    Op::Load { ptr } => {
                        self.check_value_defined(*ptr, inst.span);
                    }
                    Op::Call { args, .. } => {
                        for a in args {
                            self.check_value_defined(*a, inst.span);
                        }
                    }
                    Op::NewObject { args, .. } => {
                        for a in args {
                            self.check_value_defined(*a, inst.span);
                        }
                    }
                    Op::ConstInt(_) | Op::ConstFloat(_) | Op::ConstBool(_) | Op::ConstString(_) | Op::Alloc { .. } => {}
                }

                self.defined_values.insert(inst.result, inst.ty.clone());
            }

            // Check terminators (AIR-002: CFG validity & AIR-003: Type validity)
            match &block.terminator {
                Terminator::Return(val, span) => {
                    if let Some(v) = val {
                        self.check_value_defined(*v, *span);
                        if let Some(v_ty) = self.defined_values.get(v) {
                            if !v_ty.is_assignable_to(&self.func.return_type) {
                                self.diagnostics.push(Diagnostic::error(
                                    "AIR-003",
                                    format!("Return type mismatch: expected '{}', found '{}'", self.func.return_type, v_ty),
                                    *span,
                                ));
                            }
                        }
                    } else if self.func.return_type != Type::Void {
                        self.diagnostics.push(Diagnostic::error(
                            "AIR-003",
                            format!("Return type mismatch: expected '{}', found void", self.func.return_type),
                            *span,
                        ));
                    }
                }
                Terminator::Branch(target, args, span) => {
                    if !block_map.contains_key(target) {
                        self.diagnostics.push(Diagnostic::error(
                            "AIR-002",
                            format!("Branch target {} does not exist", target),
                            *span,
                        ));
                    }
                    for a in args {
                        self.check_value_defined(*a, *span);
                    }
                }
                Terminator::CondBranch {
                    cond,
                    then_block,
                    then_args,
                    else_block,
                    else_args,
                    span,
                } => {
                    self.check_value_defined(*cond, *span);
                    if let Some(c_ty) = self.defined_values.get(cond) {
                        if *c_ty != Type::Bool {
                            self.diagnostics.push(Diagnostic::error(
                                "AIR-003",
                                format!("Branch condition must be 'bool', found '{}'", c_ty),
                                *span,
                            ));
                        }
                    }
                    if !block_map.contains_key(then_block) {
                        self.diagnostics.push(Diagnostic::error(
                            "AIR-002",
                            format!("Then branch target {} does not exist", then_block),
                            *span,
                        ));
                    }
                    if !block_map.contains_key(else_block) {
                        self.diagnostics.push(Diagnostic::error(
                            "AIR-002",
                            format!("Else branch target {} does not exist", else_block),
                            *span,
                        ));
                    }
                    for a in then_args {
                        self.check_value_defined(*a, *span);
                    }
                    for a in else_args {
                        self.check_value_defined(*a, *span);
                    }
                }
            }
        }

        if self.diagnostics.is_empty() {
            Ok(())
        } else {
            Err(self.diagnostics)
        }
    }

    fn check_value_defined(&mut self, val: ValueId, span: Span) {
        if !self.defined_values.contains_key(&val) {
            self.diagnostics.push(Diagnostic::error(
                "AIR-001",
                format!("SSA violation: value {} is used before definition", val),
                span,
            ));
        }
    }
}
