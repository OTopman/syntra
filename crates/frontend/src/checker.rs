use crate::symbols::*;
use crate::types::Type;
use aether_diagnostics::{Diagnostic, Span};
use aether_syntax::ast::*;
use std::collections::HashMap;

pub struct TypeChecker {
    symbols: SymbolTable,
    diagnostics: Vec<Diagnostic>,
    current_return_type: Option<Type>,
    current_class: Option<String>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
            diagnostics: Vec::new(),
            current_return_type: None,
            current_class: None,
        }
    }

    pub fn check_program(mut self, program: &Program) -> Result<(), Vec<Diagnostic>> {
        // Pass 1: Collect all top-level symbols (functions, classes, interfaces)
        for item in &program.items {
            match item {
                Item::Interface(iface) => self.register_interface(iface),
                Item::Class(cls) => self.register_class(cls),
                Item::Function(func) => self.register_function(func),
            }
        }

        // Pass 2: Check interface conformance for all classes
        for item in &program.items {
            if let Item::Class(cls) = item {
                self.check_interface_conformance(cls);
            }
        }

        // Pass 3: Type check all bodies (functions, methods, constructors)
        for item in &program.items {
            match item {
                Item::Function(func) => self.check_function(func),
                Item::Class(cls) => self.check_class_body(cls),
                Item::Interface(_) => {}
            }
        }

        if self.diagnostics.is_empty() {
            Ok(())
        } else {
            Err(self.diagnostics)
        }
    }

    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic::error("LANG-020", message, span));
    }

    // --- Pass 1: Registration ---
    fn register_interface(&mut self, iface: &InterfaceDecl) {
        let mut methods = HashMap::new();
        for sig in &iface.methods {
            let params = sig
                .params
                .iter()
                .map(|p| (p.name.clone(), Type::from_ast(&p.ty)))
                .collect();
            let return_type = sig
                .return_type
                .as_ref()
                .map(Type::from_ast)
                .unwrap_or(Type::Void);

            methods.insert(
                sig.name.clone(),
                MethodSymbol {
                    name: sig.name.clone(),
                    params,
                    return_type,
                    is_public: true,
                    is_static: false,
                    is_override: false,
                    span: sig.span,
                },
            );
        }

        if self.symbols.interfaces.contains_key(&iface.name) {
            self.error(format!("Duplicate interface '{}'", iface.name), iface.span);
        } else {
            self.symbols.interfaces.insert(
                iface.name.clone(),
                InterfaceSymbol {
                    name: iface.name.clone(),
                    methods,
                    span: iface.span,
                },
            );
        }
    }

    fn register_class(&mut self, cls: &ClassDecl) {
        let mut fields = HashMap::new();
        let mut methods = HashMap::new();
        let mut constructor = None;

        for member in &cls.members {
            match member {
                ClassMember::Field(f) => {
                    let field = FieldSymbol {
                        name: f.name.clone(),
                        ty: Type::from_ast(&f.ty),
                        is_public: f.is_public,
                        is_static: f.is_static,
                        span: f.span,
                    };
                    if fields.contains_key(&f.name) {
                        self.error(format!("Duplicate field '{}' in class '{}'", f.name, cls.name), f.span);
                    } else {
                        fields.insert(f.name.clone(), field);
                    }
                }
                ClassMember::Constructor(c) => {
                    let params = c
                        .params
                        .iter()
                        .map(|p| (p.name.clone(), Type::from_ast(&p.ty)))
                        .collect();
                    constructor = Some(ConstructorSymbol {
                        params,
                        span: c.span,
                    });
                }
                ClassMember::Method(m) => {
                    let params = m
                        .func
                        .params
                        .iter()
                        .map(|p| (p.name.clone(), Type::from_ast(&p.ty)))
                        .collect();
                    let return_type = m
                        .func
                        .return_type
                        .as_ref()
                        .map(Type::from_ast)
                        .unwrap_or(Type::Void);

                    let method = MethodSymbol {
                        name: m.func.name.clone(),
                        params,
                        return_type,
                        is_public: m.is_public,
                        is_static: m.is_static,
                        is_override: m.is_override,
                        span: m.span,
                    };
                    if methods.contains_key(&m.func.name) {
                        self.error(format!("Duplicate method '{}' in class '{}'", m.func.name, cls.name), m.span);
                    } else {
                        methods.insert(m.func.name.clone(), method);
                    }
                }
            }
        }

        if self.symbols.classes.contains_key(&cls.name) {
            self.error(format!("Duplicate class '{}'", cls.name), cls.span);
        } else {
            self.symbols.classes.insert(
                cls.name.clone(),
                ClassSymbol {
                    name: cls.name.clone(),
                    extends: cls.extends.clone(),
                    implements: cls.implements.clone(),
                    fields,
                    methods,
                    constructor,
                    span: cls.span,
                },
            );
        }
    }

    fn register_function(&mut self, func: &FunctionDecl) {
        let params = func
            .params
            .iter()
            .map(|p| (p.name.clone(), Type::from_ast(&p.ty)))
            .collect();
        let return_type = func
            .return_type
            .as_ref()
            .map(Type::from_ast)
            .unwrap_or(Type::Void);

        if self.symbols.functions.contains_key(&func.name) {
            self.error(format!("Duplicate function '{}'", func.name), func.span);
        } else {
            self.symbols.functions.insert(
                func.name.clone(),
                FunctionSymbol {
                    name: func.name.clone(),
                    params,
                    return_type,
                    span: func.span,
                },
            );
        }
    }

    // --- Pass 2: Interface Conformance ---
    fn check_interface_conformance(&mut self, cls: &ClassDecl) {
        let class_sym = match self.symbols.classes.get(&cls.name).cloned() {
            Some(c) => c,
            None => return,
        };

        for iface_name in &class_sym.implements {
            let iface_sym = match self.symbols.interfaces.get(iface_name).cloned() {
                Some(i) => i,
                None => {
                    self.error(format!("Class '{}' implements undefined interface '{}'", cls.name, iface_name), cls.span);
                    continue;
                }
            };

            for (m_name, req_m) in &iface_sym.methods {
                match class_sym.methods.get(m_name) {
                    Some(cls_m) => {
                        if cls_m.return_type != req_m.return_type {
                            self.error(
                                format!(
                                    "Class '{}' implements interface '{}' but method '{}' return type '{}' does not match expected '{}'",
                                    cls.name, iface_name, m_name, cls_m.return_type, req_m.return_type
                                ),
                                cls_m.span,
                            );
                        }
                        if cls_m.params.len() != req_m.params.len() {
                            self.error(
                                format!(
                                    "Class '{}' implements interface '{}' but method '{}' has {} parameters (expected {})",
                                    cls.name, iface_name, m_name, cls_m.params.len(), req_m.params.len()
                                ),
                                cls_m.span,
                            );
                        } else {
                            for ((_, p_cls), (_, p_req)) in cls_m.params.iter().zip(&req_m.params) {
                                if p_cls != p_req {
                                    self.error(
                                        format!(
                                            "Class '{}' method '{}' parameter type '{}' does not match interface contract '{}'",
                                            cls.name, m_name, p_cls, p_req
                                        ),
                                        cls_m.span,
                                    );
                                }
                            }
                        }
                    }
                    None => {
                        self.error(
                            format!(
                                "Class '{}' promises to implement interface '{}' but is missing required method '{}'",
                                cls.name, iface_name, m_name
                            ),
                            cls.span,
                        );
                    }
                }
            }
        }
    }

    // --- Pass 3: Body & Expression Checking ---
    fn check_class_body(&mut self, cls: &ClassDecl) {
        self.current_class = Some(cls.name.clone());

        for member in &cls.members {
            match member {
                ClassMember::Constructor(c) => {
                    self.symbols.enter_scope();
                    // Inject parameters into scope
                    for p in &c.params {
                        let ty = Type::from_ast(&p.ty);
                        let _ = self.symbols.define_var(VariableSymbol {
                            name: p.name.clone(),
                            ty,
                            is_const: false,
                            span: p.span,
                        });
                    }
                    self.current_return_type = Some(Type::Void);
                    self.check_block(&c.body);
                    self.symbols.exit_scope();
                }
                ClassMember::Method(m) => {
                    self.check_function(&m.func);
                }
                ClassMember::Field(f) => {
                    if let Some(init) = &f.init {
                        let init_ty = self.check_expression(init);
                        let field_ty = Type::from_ast(&f.ty);
                        if !init_ty.is_assignable_to(&field_ty) {
                            self.error(
                                format!("Cannot initialize field '{}' of type '{}' with value of type '{}'", f.name, field_ty, init_ty),
                                f.span,
                            );
                        }
                    }
                }
            }
        }

        self.current_class = None;
    }

    fn check_function(&mut self, func: &FunctionDecl) {
        let return_type = func
            .return_type
            .as_ref()
            .map(Type::from_ast)
            .unwrap_or(Type::Void);

        self.current_return_type = Some(return_type);
        self.symbols.enter_scope();

        // Inject parameters into scope
        for p in &func.params {
            let ty = Type::from_ast(&p.ty);
            if let Err(existing) = self.symbols.define_var(VariableSymbol {
                name: p.name.clone(),
                ty,
                is_const: false,
                span: p.span,
            }) {
                self.error(format!("Duplicate parameter name '{}' in function '{}'", p.name, func.name), existing.span);
            }
        }

        self.check_block(&func.body);
        self.symbols.exit_scope();
        self.current_return_type = None;
    }

    fn check_block(&mut self, block: &Block) {
        for stmt in &block.statements {
            self.check_statement(stmt);
        }
    }

    fn check_statement(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl { is_const, ty, name, init, span } => {
                let declared_ty = Type::from_ast(ty);
                let init_ty = self.check_expression(init);

                if !init_ty.is_assignable_to(&declared_ty) {
                    self.error(
                        format!("Cannot assign value of type '{}' to variable '{}' of type '{}'", init_ty, name, declared_ty),
                        *span,
                    );
                }

                if let Err(existing) = self.symbols.define_var(VariableSymbol {
                    name: name.clone(),
                    ty: declared_ty,
                    is_const: *is_const,
                    span: *span,
                }) {
                    self.error(format!("Variable '{}' is already defined in this scope", name), existing.span);
                }
            }
            Stmt::Assignment { target, value, span } => {
                let target_ty = self.check_expression(target);
                let value_ty = self.check_expression(value);

                if !value_ty.is_assignable_to(&target_ty) {
                    self.error(
                        format!("Cannot assign value of type '{}' to target of type '{}'", value_ty, target_ty),
                        *span,
                    );
                }

                // Check const re-assignment
                if let Expr::Variable(name, _) = target {
                    if let Some(var) = self.symbols.lookup_var(name) {
                        if var.is_const {
                            self.error(format!("Cannot reassign to constant variable '{}'", name), *span);
                        }
                    }
                }
            }
            Stmt::If { condition, then_branch, else_branch, span } => {
                let cond_ty = self.check_expression(condition);
                if cond_ty != Type::Bool && cond_ty != Type::Unknown {
                    self.error(format!("'if' condition must evaluate to 'bool', found '{}'", cond_ty), *span);
                }
                self.symbols.enter_scope();
                self.check_block(then_branch);
                self.symbols.exit_scope();

                if let Some(else_b) = else_branch {
                    self.symbols.enter_scope();
                    self.check_block(else_b);
                    self.symbols.exit_scope();
                }
            }
            Stmt::While { condition, body, span } => {
                let cond_ty = self.check_expression(condition);
                if cond_ty != Type::Bool && cond_ty != Type::Unknown {
                    self.error(format!("'while' condition must evaluate to 'bool', found '{}'", cond_ty), *span);
                }
                self.symbols.enter_scope();
                self.check_block(body);
                self.symbols.exit_scope();
            }
            Stmt::Return { value, span } => {
                let val_ty = value
                    .as_ref()
                    .map(|e| self.check_expression(e))
                    .unwrap_or(Type::Void);

                if let Some(expected) = &self.current_return_type {
                    if !val_ty.is_assignable_to(expected) {
                        self.error(
                            format!("Return type mismatch: expected '{}', found '{}'", expected, val_ty),
                            *span,
                        );
                    }
                }
            }
            Stmt::Expr { expr, .. } => {
                self.check_expression(expr);
            }
        }
    }

    fn check_expression(&mut self, expr: &Expr) -> Type {
        match expr {
            Expr::Integer(_, _) => Type::Int,
            Expr::Float(_, _) => Type::Float,
            Expr::StringLit(_, _) => Type::String,
            Expr::BoolLit(_, _) => Type::Bool,
            Expr::Variable(name, span) => {
                if let Some(var) = self.symbols.lookup_var(name) {
                    var.ty.clone()
                } else if self.current_class.is_some() {
                    // Check if it's a field on this
                    let cls_name = self.current_class.as_ref().unwrap();
                    if let Some(cls) = self.symbols.classes.get(cls_name) {
                        if let Some(field) = cls.fields.get(name) {
                            return field.ty.clone();
                        }
                    }
                    self.error(format!("Undefined variable or field '{}'", name), *span);
                    Type::Unknown
                } else {
                    self.error(format!("Undefined variable '{}'", name), *span);
                    Type::Unknown
                }
            }
            Expr::This(span) => {
                if let Some(cls_name) = &self.current_class {
                    Type::Class(cls_name.clone())
                } else {
                    self.error("'this' can only be used inside a class method or constructor", *span);
                    Type::Unknown
                }
            }
            Expr::Binary { op, left, right, span } => {
                let left_ty = self.check_expression(left);
                let right_ty = self.check_expression(right);

                match op {
                    BinaryOp::Add => {
                        if left_ty == Type::Int && right_ty == Type::Int {
                            Type::Int
                        } else if left_ty == Type::Float && right_ty == Type::Float {
                            Type::Float
                        } else if left_ty == Type::String || right_ty == Type::String {
                            Type::String // String concatenation!
                        } else {
                            self.error(format!("Cannot add types '{}' and '{}'", left_ty, right_ty), *span);
                            Type::Unknown
                        }
                    }
                    BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                        if left_ty == Type::Int && right_ty == Type::Int {
                            Type::Int
                        } else if left_ty == Type::Float && right_ty == Type::Float {
                            Type::Float
                        } else {
                            self.error(format!("Arithmetic operation requires numeric types, found '{}' and '{}'", left_ty, right_ty), *span);
                            Type::Unknown
                        }
                    }
                    BinaryOp::EqualEqual | BinaryOp::NotEqual => {
                        if left_ty.is_assignable_to(&right_ty) || right_ty.is_assignable_to(&left_ty) {
                            Type::Bool
                        } else {
                            self.error(format!("Cannot compare equality between unrelated types '{}' and '{}'", left_ty, right_ty), *span);
                            Type::Bool
                        }
                    }
                    BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
                        if left_ty.is_numeric() && right_ty.is_numeric() {
                            Type::Bool
                        } else {
                            self.error(format!("Ordering comparison requires numbers, found '{}' and '{}'", left_ty, right_ty), *span);
                            Type::Bool
                        }
                    }
                }
            }
            Expr::Call { callee, args, span } => {
                match callee.as_ref() {
                    Expr::Variable(fn_name, fn_span) => {
                        // Built-in functions
                        if fn_name == "println" || fn_name == "print" {
                            for arg in args {
                                self.check_expression(arg);
                            }
                            return Type::Void;
                        }

                        if let Some(func) = self.symbols.functions.get(fn_name).cloned() {
                            if func.params.len() != args.len() {
                                self.error(
                                    format!("Function '{}' expects {} arguments, but got {}", fn_name, func.params.len(), args.len()),
                                    *span,
                                );
                            } else {
                                for (arg_expr, (_, param_ty)) in args.iter().zip(&func.params) {
                                    let arg_ty = self.check_expression(arg_expr);
                                    if !arg_ty.is_assignable_to(param_ty) {
                                        self.error(
                                            format!("Argument type '{}' does not match parameter type '{}'", arg_ty, param_ty),
                                            arg_expr.span(),
                                        );
                                    }
                                }
                            }
                            func.return_type
                        } else {
                            self.error(format!("Call to undefined function '{}'", fn_name), *fn_span);
                            Type::Unknown
                        }
                    }
                    Expr::MemberAccess { object, member, span: m_span } => {
                        let obj_ty = self.check_expression(object);
                        if let Type::Class(cls_name) = &obj_ty {
                            if let Some(cls) = self.symbols.classes.get(cls_name).cloned() {
                                if let Some(m) = cls.methods.get(member) {
                                    if m.params.len() != args.len() {
                                        self.error(
                                            format!("Method '{}' on class '{}' expects {} arguments, but got {}", member, cls_name, m.params.len(), args.len()),
                                            *span,
                                        );
                                    } else {
                                        for (arg_expr, (_, param_ty)) in args.iter().zip(&m.params) {
                                            let arg_ty = self.check_expression(arg_expr);
                                            if !arg_ty.is_assignable_to(param_ty) {
                                                self.error(
                                                    format!("Argument type '{}' does not match method parameter type '{}'", arg_ty, param_ty),
                                                    arg_expr.span(),
                                                );
                                            }
                                        }
                                    }
                                    return m.return_type.clone();
                                } else {
                                    self.error(format!("No method named '{}' found on class '{}'", member, cls_name), *m_span);
                                    return Type::Unknown;
                                }
                            }
                        }
                        self.error(format!("Cannot invoke method '{}' on non-class type '{}'", member, obj_ty), *span);
                        Type::Unknown
                    }
                    _ => {
                        self.error("Invalid call expression", *span);
                        Type::Unknown
                    }
                }
            }
            Expr::MemberAccess { object, member, span } => {
                let obj_ty = self.check_expression(object);
                if let Type::Class(cls_name) = &obj_ty {
                    if let Some(cls) = self.symbols.classes.get(cls_name) {
                        if let Some(field) = cls.fields.get(member) {
                            return field.ty.clone();
                        } else {
                            self.error(format!("Field '{}' not found on class '{}'", member, cls_name), *span);
                            return Type::Unknown;
                        }
                    }
                }
                self.error(format!("Cannot access property '{}' on non-class type '{}'", member, obj_ty), *span);
                Type::Unknown
            }
            Expr::New { class_name, args, span } => {
                if let Some(cls) = self.symbols.classes.get(class_name).cloned() {
                    if let Some(ctor) = &cls.constructor {
                        if ctor.params.len() != args.len() {
                            self.error(
                                format!("Constructor for '{}' expects {} arguments, but got {}", class_name, ctor.params.len(), args.len()),
                                *span,
                            );
                        } else {
                            for (arg_expr, (_, param_ty)) in args.iter().zip(&ctor.params) {
                                let arg_ty = self.check_expression(arg_expr);
                                if !arg_ty.is_assignable_to(param_ty) {
                                    self.error(
                                        format!("Constructor argument type '{}' does not match parameter type '{}'", arg_ty, param_ty),
                                        arg_expr.span(),
                                    );
                                }
                            }
                        }
                    } else if !args.is_empty() {
                        self.error(format!("Class '{}' has no constructor taking {} arguments", class_name, args.len()), *span);
                    }
                    Type::Class(class_name.clone())
                } else {
                    self.error(format!("Cannot instantiate undefined class '{}'", class_name), *span);
                    Type::Unknown
                }
            }
        }
    }
}
