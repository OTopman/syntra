use aether_syntax::ast::TypeRef;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Void,
    Array(Box<Type>),
    Class(String),
    Interface(String),
    Unknown,
}

impl Type {
    pub fn from_ast(type_ref: &TypeRef) -> Self {
        let base = match type_ref.name.as_str() {
            "int" => Type::Int,
            "float" => Type::Float,
            "bool" => Type::Bool,
            "string" => Type::String,
            "void" => Type::Void,
            custom => Type::Class(custom.to_string()),
        };

        if type_ref.is_array {
            Type::Array(Box::new(base))
        } else {
            base
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float)
    }

    pub fn is_assignable_to(&self, target: &Type) -> bool {
        if self == target || *self == Type::Unknown || *target == Type::Unknown {
            return true;
        }

        // Float can accept Int promotion if desired, or strictly equal
        if let (Type::Array(src_elem), Type::Array(dst_elem)) = (self, target) {
            return src_elem.is_assignable_to(dst_elem);
        }

        false
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Bool => write!(f, "bool"),
            Type::String => write!(f, "string"),
            Type::Void => write!(f, "void"),
            Type::Array(elem) => write!(f, "{}[]", elem),
            Type::Class(name) => write!(f, "{}", name),
            Type::Interface(name) => write!(f, "{}", name),
            Type::Unknown => write!(f, "<unknown>"),
        }
    }
}
