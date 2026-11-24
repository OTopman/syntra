use crate::types::Type;
use std::collections::HashMap;
use syntra_diagnostics::Span;

#[derive(Debug, Clone)]
pub struct VariableSymbol {
    pub name: String,
    pub ty: Type,
    pub is_const: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FunctionSymbol {
    pub name: String,
    pub generic_params: Vec<String>,
    pub params: Vec<(String, Type)>,
    pub return_type: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FieldSymbol {
    pub name: String,
    pub ty: Type,
    pub is_public: bool,
    pub is_static: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MethodSymbol {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub return_type: Type,
    pub is_public: bool,
    pub is_static: bool,
    pub is_override: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ConstructorSymbol {
    pub params: Vec<(String, Type)>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ClassSymbol {
    pub name: String,
    pub generic_params: Vec<String>,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub fields: HashMap<String, FieldSymbol>,
    pub methods: HashMap<String, MethodSymbol>,
    pub constructor: Option<ConstructorSymbol>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct InterfaceSymbol {
    pub name: String,
    pub methods: HashMap<String, MethodSymbol>,
    pub span: Span,
}

#[derive(Debug, Default)]
pub struct Scope {
    pub variables: HashMap<String, VariableSymbol>,
}

#[derive(Debug, Default)]
pub struct SymbolTable {
    pub classes: HashMap<String, ClassSymbol>,
    pub interfaces: HashMap<String, InterfaceSymbol>,
    pub functions: HashMap<String, FunctionSymbol>,
    pub scopes: Vec<Scope>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            classes: HashMap::new(),
            interfaces: HashMap::new(),
            functions: HashMap::new(),
            scopes: vec![Scope::default()],
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn define_var(&mut self, var: VariableSymbol) -> Result<(), VariableSymbol> {
        if let Some(current) = self.scopes.last_mut() {
            if let Some(existing) = current.variables.get(&var.name) {
                return Err(existing.clone());
            }
            current.variables.insert(var.name.clone(), var);
        }
        Ok(())
    }

    pub fn lookup_var(&self, name: &str) -> Option<&VariableSymbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(var) = scope.variables.get(name) {
                return Some(var);
            }
        }
        None
    }
}
