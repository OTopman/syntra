use crate::ir::*;
use aether_frontend::Type;
use aether_syntax::ast::*;
use std::collections::HashMap;

pub struct Lowerer {
    next_val: usize,
    next_block: usize,
    current_instructions: Vec<Instruction>,
    blocks: Vec<BasicBlock>,
    variables: HashMap<String, ValueId>,
}

impl Lowerer {
    pub fn new() -> Self {
        Self {
            next_val: 0,
            next_block: 0,
            current_instructions: Vec::new(),
            blocks: Vec::new(),
            variables: HashMap::new(),
        }
    }

    fn alloc_val(&mut self) -> ValueId {
        let id = ValueId(self.next_val);
        self.next_val += 1;
        id
    }

    fn alloc_block(&mut self) -> BlockId {
        let id = BlockId(self.next_block);
        self.next_block += 1;
        id
    }

    pub fn lower_program(&mut self, program: &Program) -> AirModule {
        let mut functions = Vec::new();

        for item in &program.items {
            match item {
                Item::Function(func) => {
                    functions.push(self.lower_function(func));
                }
                Item::Class(cls) => {
                    for member in &cls.members {
                        if let ClassMember::Method(m) = member {
                            let mut method_func = m.func.clone();
                            method_func.name = format!("{}_{}", cls.name, m.func.name);
                            functions.push(self.lower_function(&method_func));
                        }
                    }
                }
                Item::Interface(_) => {}
            }
        }

        AirModule {
            name: "main_module".to_string(),
            functions,
        }
    }

    fn lower_function(&mut self, func: &FunctionDecl) -> AirFunction {
        self.blocks.clear();
        self.current_instructions.clear();
        self.variables.clear();

        let entry_id = self.alloc_block();
        let mut params = Vec::new();

        for p in &func.params {
            let v_id = self.alloc_val();
            let ty = Type::from_ast(&p.ty);
            self.variables.insert(p.name.clone(), v_id);
            params.push(AirParam {
                id: v_id,
                name: p.name.clone(),
                ty,
            });
        }

        let return_type = func
            .return_type
            .as_ref()
            .map(Type::from_ast)
            .unwrap_or(Type::Void);

        // Lower body
        let last_val = self.lower_block(&func.body);

        // Ensure terminator
        let terminator = Terminator::Return(last_val, func.span);
        let entry_block = BasicBlock {
            id: entry_id,
            params: Vec::new(),
            instructions: std::mem::take(&mut self.current_instructions),
            terminator,
        };
        self.blocks.push(entry_block);

        AirFunction {
            name: func.name.clone(),
            params,
            return_type,
            blocks: std::mem::take(&mut self.blocks),
            entry_block: entry_id,
        }
    }

    fn lower_block(&mut self, block: &Block) -> Option<ValueId> {
        let mut last_return = None;
        for stmt in &block.statements {
            match stmt {
                Stmt::VarDecl { name, init, .. } => {
                    let val = self.lower_expression(init);
                    self.variables.insert(name.clone(), val);
                }
                Stmt::Assignment { target, value, .. } => {
                    let val = self.lower_expression(value);
                    if let Expr::Variable(name, _) = target {
                        self.variables.insert(name.clone(), val);
                    }
                }
                Stmt::Return { value, .. } => {
                    let val = value.as_ref().map(|v| self.lower_expression(v));
                    last_return = val;
                }
                Stmt::Expr { expr, .. } => {
                    self.lower_expression(expr);
                }
                Stmt::If { condition, then_branch, else_branch, .. } => {
                    let _cond_val = self.lower_expression(condition);
                    let then_ret = self.lower_block(then_branch);
                    let else_ret = else_branch.as_ref().and_then(|eb| self.lower_block(eb));
                    if then_ret.is_some() {
                        last_return = then_ret;
                    } else if else_ret.is_some() {
                        last_return = else_ret;
                    }
                }
                Stmt::While { condition, body, .. } => {
                    let _cond_val = self.lower_expression(condition);
                    let body_ret = self.lower_block(body);
                    if body_ret.is_some() {
                        last_return = body_ret;
                    }
                }
            }
        }
        last_return
    }

    fn lower_expression(&mut self, expr: &Expr) -> ValueId {
        match expr {
            Expr::Integer(i, span) => {
                let res = self.alloc_val();
                self.current_instructions.push(Instruction {
                    result: res,
                    ty: Type::Int,
                    op: Op::ConstInt(*i),
                    span: *span,
                });
                res
            }
            Expr::Float(f, span) => {
                let res = self.alloc_val();
                self.current_instructions.push(Instruction {
                    result: res,
                    ty: Type::Float,
                    op: Op::ConstFloat(*f),
                    span: *span,
                });
                res
            }
            Expr::BoolLit(b, span) => {
                let res = self.alloc_val();
                self.current_instructions.push(Instruction {
                    result: res,
                    ty: Type::Bool,
                    op: Op::ConstBool(*b),
                    span: *span,
                });
                res
            }
            Expr::StringLit(s, span) => {
                let res = self.alloc_val();
                self.current_instructions.push(Instruction {
                    result: res,
                    ty: Type::String,
                    op: Op::ConstString(s.clone()),
                    span: *span,
                });
                res
            }
            Expr::Variable(name, span) => {
                if let Some(id) = self.variables.get(name) {
                    *id
                } else {
                    let res = self.alloc_val();
                    self.current_instructions.push(Instruction {
                        result: res,
                        ty: Type::Unknown,
                        op: Op::ConstInt(0),
                        span: *span,
                    });
                    res
                }
            }
            Expr::Binary { op, left, right, span } => {
                let l_val = self.lower_expression(left);
                let r_val = self.lower_expression(right);
                let res = self.alloc_val();

                let ty = match op {
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => Type::Int,
                    _ => Type::Bool,
                };

                self.current_instructions.push(Instruction {
                    result: res,
                    ty,
                    op: Op::Binary {
                        op: *op,
                        left: l_val,
                        right: r_val,
                    },
                    span: *span,
                });
                res
            }
            Expr::Call { callee, args, span } => {
                let mut arg_vals = Vec::new();
                for a in args {
                    arg_vals.push(self.lower_expression(a));
                }

                let fn_name = match callee.as_ref() {
                    Expr::Variable(name, _) => name.clone(),
                    Expr::MemberAccess { member, .. } => member.clone(),
                    _ => "unknown_fn".to_string(),
                };

                let res = self.alloc_val();
                self.current_instructions.push(Instruction {
                    result: res,
                    ty: Type::Int,
                    op: Op::Call {
                        func_name: fn_name,
                        args: arg_vals,
                    },
                    span: *span,
                });
                res
            }
            Expr::New { class_name, args, span } => {
                let mut arg_vals = Vec::new();
                for a in args {
                    arg_vals.push(self.lower_expression(a));
                }
                let res = self.alloc_val();
                self.current_instructions.push(Instruction {
                    result: res,
                    ty: Type::Class(class_name.clone()),
                    op: Op::NewObject {
                        class_name: class_name.clone(),
                        domain: MemoryDomain::Stack, // Stack-allocated by default!
                        args: arg_vals,
                    },
                    span: *span,
                });
                res
            }
            Expr::MemberAccess { object, .. } => {
                self.lower_expression(object)
            }
            Expr::This(span) => {
                let res = self.alloc_val();
                self.current_instructions.push(Instruction {
                    result: res,
                    ty: Type::Unknown,
                    op: Op::ConstInt(0),
                    span: *span,
                });
                res
            }
        }
    }
}
