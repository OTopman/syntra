use crate::value::{Instance, Value};
use aether_diagnostics::Span;
use aether_syntax::ast::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Runtime error at span {span:?}: {message}")]
    General { message: String, span: Span },
}

enum ControlFlow {
    Return(Value),
    Error(RuntimeError),
}

pub struct Interpreter {
    classes: HashMap<String, ClassDecl>,
    functions: HashMap<String, FunctionDecl>,
    scopes: Vec<HashMap<String, Value>>,
    current_instance: Option<Rc<RefCell<Instance>>>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
            functions: HashMap::new(),
            scopes: vec![HashMap::new()],
            current_instance: None,
        }
    }

    pub fn run_program(&mut self, program: &Program) -> Result<Value, RuntimeError> {
        // Register top-level items
        for item in &program.items {
            match item {
                Item::Function(f) => {
                    self.functions.insert(f.name.clone(), f.clone());
                }
                Item::Class(c) => {
                    self.classes.insert(c.name.clone(), c.clone());
                }
                Item::Interface(_) => {}
            }
        }

        // Look for main function
        let main_func = self
            .functions
            .get("main")
            .cloned()
            .ok_or_else(|| RuntimeError::General {
                message: "No 'main' function found in program".to_string(),
                span: Span::default(),
            })?;

        match self.call_function(&main_func, Vec::new()) {
            Ok(val) => Ok(val),
            Err(ControlFlow::Return(val)) => Ok(val),
            Err(ControlFlow::Error(err)) => Err(err),
        }
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn set_variable(&mut self, name: &str, val: Value) {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), val);
                return;
            }
        }
        // If not found in outer scopes, insert into innermost scope
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), val);
        }
    }

    fn get_variable(&self, name: &str) -> Option<Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.get(name) {
                return Some(val.clone());
            }
        }
        None
    }

    fn call_function(&mut self, func: &FunctionDecl, args: Vec<Value>) -> Result<Value, ControlFlow> {
        self.enter_scope();

        for (param, arg) in func.params.iter().zip(args) {
            self.set_variable(&param.name, arg);
        }

        let res = self.execute_block(&func.body);
        self.exit_scope();

        match res {
            Ok(()) => Ok(Value::Void),
            Err(ControlFlow::Return(val)) => Ok(val),
            Err(err) => Err(err),
        }
    }

    fn execute_block(&mut self, block: &Block) -> Result<(), ControlFlow> {
        for stmt in &block.statements {
            self.execute_statement(stmt)?;
        }
        Ok(())
    }

    fn execute_statement(&mut self, stmt: &Stmt) -> Result<(), ControlFlow> {
        match stmt {
            Stmt::VarDecl { name, init, .. } => {
                let val = self.eval_expr(init).map_err(ControlFlow::Error)?;
                if let Some(scope) = self.scopes.last_mut() {
                    scope.insert(name.clone(), val);
                }
            }
            Stmt::Assignment { target, value, span } => {
                let val = self.eval_expr(value).map_err(ControlFlow::Error)?;
                match target {
                    Expr::Variable(name, _) => {
                        self.set_variable(name, val);
                    }
                    Expr::MemberAccess { object, member, .. } => {
                        let obj_val = self.eval_expr(object).map_err(ControlFlow::Error)?;
                        if let Value::Object(inst) = obj_val {
                            inst.borrow_mut().fields.insert(member.clone(), val);
                        } else {
                            return Err(ControlFlow::Error(RuntimeError::General {
                                message: format!("Cannot set property '{}' on non-object", member),
                                span: *span,
                            }));
                        }
                    }
                    _ => {
                        return Err(ControlFlow::Error(RuntimeError::General {
                            message: "Invalid assignment target".to_string(),
                            span: *span,
                        }));
                    }
                }
            }
            Stmt::If { condition, then_branch, else_branch, .. } => {
                let cond_val = self.eval_expr(condition).map_err(ControlFlow::Error)?;
                if cond_val.is_truthy() {
                    self.enter_scope();
                    let res = self.execute_block(then_branch);
                    self.exit_scope();
                    res?;
                } else if let Some(else_b) = else_branch {
                    self.enter_scope();
                    let res = self.execute_block(else_b);
                    self.exit_scope();
                    res?;
                }
            }
            Stmt::While { condition, body, .. } => {
                while self.eval_expr(condition).map_err(ControlFlow::Error)?.is_truthy() {
                    self.enter_scope();
                    let res = self.execute_block(body);
                    self.exit_scope();
                    res?;
                }
            }
            Stmt::Return { value, .. } => {
                let ret_val = if let Some(v) = value {
                    self.eval_expr(v).map_err(ControlFlow::Error)?
                } else {
                    Value::Void
                };
                return Err(ControlFlow::Return(ret_val));
            }
            Stmt::Expr { expr, .. } => {
                self.eval_expr(expr).map_err(ControlFlow::Error)?;
            }
        }
        Ok(())
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Integer(i, _) => Ok(Value::Int(*i)),
            Expr::Float(f, _) => Ok(Value::Float(*f)),
            Expr::StringLit(s, _) => Ok(Value::String(s.clone())),
            Expr::BoolLit(b, _) => Ok(Value::Bool(*b)),
            Expr::This(span) => {
                if let Some(inst) = &self.current_instance {
                    Ok(Value::Object(inst.clone()))
                } else {
                    Err(RuntimeError::General {
                        message: "'this' used outside of an instance method".to_string(),
                        span: *span,
                    })
                }
            }
            Expr::Variable(name, span) => {
                if let Some(val) = self.get_variable(name) {
                    Ok(val)
                } else if let Some(inst) = &self.current_instance {
                    if let Some(val) = inst.borrow().fields.get(name) {
                        return Ok(val.clone());
                    }
                    Err(RuntimeError::General {
                        message: format!("Undefined variable or field '{}'", name),
                        span: *span,
                    })
                } else {
                    Err(RuntimeError::General {
                        message: format!("Undefined variable '{}'", name),
                        span: *span,
                    })
                }
            }
            Expr::Binary { op, left, right, span } => {
                let l_val = self.eval_expr(left)?;
                let r_val = self.eval_expr(right)?;

                match op {
                    BinaryOp::Add => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l + r)),
                        (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l + r)),
                        (l, r) => Ok(Value::String(format!("{}{}", l, r))),
                    },
                    BinaryOp::Sub => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l - r)),
                        (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l - r)),
                        _ => Err(RuntimeError::General {
                            message: "Cannot subtract non-numeric values".to_string(),
                            span: *span,
                        }),
                    },
                    BinaryOp::Mul => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l * r)),
                        (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l * r)),
                        _ => Err(RuntimeError::General {
                            message: "Cannot multiply non-numeric values".to_string(),
                            span: *span,
                        }),
                    },
                    BinaryOp::Div => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => {
                            if r == 0 {
                                Err(RuntimeError::General {
                                    message: "Division by zero".to_string(),
                                    span: *span,
                                })
                            } else {
                                Ok(Value::Int(l / r))
                            }
                        }
                        (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l / r)),
                        _ => Err(RuntimeError::General {
                            message: "Cannot divide non-numeric values".to_string(),
                            span: *span,
                        }),
                    },
                    BinaryOp::EqualEqual => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l == r)),
                        (Value::Float(l), Value::Float(r)) => Ok(Value::Bool((l - r).abs() < f64::EPSILON)),
                        (Value::Bool(l), Value::Bool(r)) => Ok(Value::Bool(l == r)),
                        (Value::String(l), Value::String(r)) => Ok(Value::Bool(l == r)),
                        _ => Ok(Value::Bool(false)),
                    },
                    BinaryOp::NotEqual => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l != r)),
                        (Value::Float(l), Value::Float(r)) => Ok(Value::Bool((l - r).abs() >= f64::EPSILON)),
                        (Value::Bool(l), Value::Bool(r)) => Ok(Value::Bool(l != r)),
                        (Value::String(l), Value::String(r)) => Ok(Value::Bool(l != r)),
                        _ => Ok(Value::Bool(true)),
                    },
                    BinaryOp::Less => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l < r)),
                        (Value::Float(l), Value::Float(r)) => Ok(Value::Bool(l < r)),
                        _ => Err(RuntimeError::General {
                            message: "'<' requires numbers".to_string(),
                            span: *span,
                        }),
                    },
                    BinaryOp::LessEqual => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l <= r)),
                        (Value::Float(l), Value::Float(r)) => Ok(Value::Bool(l <= r)),
                        _ => Err(RuntimeError::General {
                            message: "'<=' requires numbers".to_string(),
                            span: *span,
                        }),
                    },
                    BinaryOp::Greater => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l > r)),
                        (Value::Float(l), Value::Float(r)) => Ok(Value::Bool(l > r)),
                        _ => Err(RuntimeError::General {
                            message: "'>' requires numbers".to_string(),
                            span: *span,
                        }),
                    },
                    BinaryOp::GreaterEqual => match (l_val, r_val) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Bool(l >= r)),
                        (Value::Float(l), Value::Float(r)) => Ok(Value::Bool(l >= r)),
                        _ => Err(RuntimeError::General {
                            message: "'>=' requires numbers".to_string(),
                            span: *span,
                        }),
                    },
                }
            }
            Expr::Call { callee, args, span } => {
                let mut evaluated_args = Vec::new();
                for arg in args {
                    evaluated_args.push(self.eval_expr(arg)?);
                }

                match callee.as_ref() {
                    Expr::Variable(fn_name, _) => {
                        // Check built-in functions
                        if fn_name == "println" {
                            if let Some(first) = evaluated_args.first() {
                                println!("{}", first);
                            } else {
                                println!();
                            }
                            return Ok(Value::Void);
                        } else if fn_name == "print" {
                            if let Some(first) = evaluated_args.first() {
                                print!("{}", first);
                            }
                            return Ok(Value::Void);
                        }

                        // User-defined function call
                        if let Some(func) = self.functions.get(fn_name).cloned() {
                            match self.call_function(&func, evaluated_args) {
                                Ok(val) => Ok(val),
                                Err(ControlFlow::Return(val)) => Ok(val),
                                Err(ControlFlow::Error(err)) => Err(err),
                            }
                        } else {
                            Err(RuntimeError::General {
                                message: format!("Call to undefined function '{}'", fn_name),
                                span: *span,
                            })
                        }
                    }
                    Expr::MemberAccess { object, member, .. } => {
                        let obj_val = self.eval_expr(object)?;
                        if let Value::Object(inst) = obj_val {
                            let cls_name = inst.borrow().class_name.clone();
                            if let Some(cls) = self.classes.get(&cls_name).cloned() {
                                for m in &cls.members {
                                    if let ClassMember::Method(method) = m {
                                        if method.func.name == *member {
                                            // Set current instance for 'this'
                                            let prev_instance = self.current_instance.take();
                                            self.current_instance = Some(inst.clone());

                                            let res = self.call_function(&method.func, evaluated_args);

                                            self.current_instance = prev_instance;

                                            return match res {
                                                Ok(val) => Ok(val),
                                                Err(ControlFlow::Return(val)) => Ok(val),
                                                Err(ControlFlow::Error(err)) => Err(err),
                                            };
                                        }
                                    }
                                }
                                return Err(RuntimeError::General {
                                    message: format!("Method '{}' not found on class '{}'", member, cls_name),
                                    span: *span,
                                });
                            }
                        }
                        Err(RuntimeError::General {
                            message: format!("Cannot invoke method '{}' on non-object", member),
                            span: *span,
                        })
                    }
                    _ => Err(RuntimeError::General {
                        message: "Invalid call expression".to_string(),
                        span: *span,
                    }),
                }
            }
            Expr::MemberAccess { object, member, span } => {
                let obj_val = self.eval_expr(object)?;
                if let Value::Object(inst) = obj_val {
                    if let Some(val) = inst.borrow().fields.get(member) {
                        return Ok(val.clone());
                    }
                    Err(RuntimeError::General {
                        message: format!("Field '{}' not found on instance of '{}'", member, inst.borrow().class_name),
                        span: *span,
                    })
                } else {
                    Err(RuntimeError::General {
                        message: format!("Cannot read property '{}' on non-object", member),
                        span: *span,
                    })
                }
            }
            Expr::New { class_name, args, span } => {
                if let Some(cls) = self.classes.get(class_name).cloned() {
                    let mut evaluated_args = Vec::new();
                    for arg in args {
                        evaluated_args.push(self.eval_expr(arg)?);
                    }

                    let instance = Rc::new(RefCell::new(Instance {
                        class_name: class_name.clone(),
                        fields: HashMap::new(),
                    }));

                    // Initialize fields with default values or initializers
                    for m in &cls.members {
                        if let ClassMember::Field(f) = m {
                            let init_val = if let Some(init) = &f.init {
                                self.eval_expr(init)?
                            } else {
                                match f.ty.name.as_str() {
                                    "int" => Value::Int(0),
                                    "float" => Value::Float(0.0),
                                    "bool" => Value::Bool(false),
                                    "string" => Value::String(String::new()),
                                    _ => Value::Void,
                                }
                            };
                            instance.borrow_mut().fields.insert(f.name.clone(), init_val);
                        }
                    }

                    // Run constructor if present
                    for m in &cls.members {
                        if let ClassMember::Constructor(ctor) = m {
                            let prev_inst = self.current_instance.take();
                            self.current_instance = Some(instance.clone());

                            self.enter_scope();
                            for (p, a) in ctor.params.iter().zip(evaluated_args.clone()) {
                                self.set_variable(&p.name, a);
                            }

                            let res = self.execute_block(&ctor.body);
                            self.exit_scope();
                            self.current_instance = prev_inst;

                            if let Err(ControlFlow::Error(err)) = res {
                                return Err(err);
                            }
                            break;
                        }
                    }

                    Ok(Value::Object(instance))
                } else {
                    Err(RuntimeError::General {
                        message: format!("Cannot instantiate undefined class '{}'", class_name),
                        span: *span,
                    })
                }
            }
        }
    }
}
