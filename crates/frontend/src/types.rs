use std::fmt;
use syntra_syntax::ast::TypeRef;

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
    GenericParam(String),
    GenericInstance(String, Vec<Type>),
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    Unknown,
}

impl Type {
    pub fn from_ast(type_ref: &TypeRef) -> Self {
        let generic_args: Vec<Type> = type_ref.generic_args.iter().map(Type::from_ast).collect();
        let base = match type_ref.name.as_str() {
            "int" => Type::Int,
            "float" => Type::Float,
            "bool" => Type::Bool,
            "string" => Type::String,
            "void" => Type::Void,
            "any" => Type::Unknown,
            custom => {
                if !generic_args.is_empty() {
                    Type::GenericInstance(custom.to_string(), generic_args)
                } else {
                    Type::Class(custom.to_string())
                }
            }
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

        if matches!(self, Type::Class(_)) && *target == Type::String {
            return true;
        }

        if let (Type::GenericParam(p1), Type::GenericParam(p2)) = (self, target) {
            return p1 == p2;
        }
        if matches!(self, Type::GenericParam(_)) || matches!(target, Type::GenericParam(_)) {
            return true;
        }

        if let (Type::Array(src_elem), Type::Array(dst_elem)) = (self, target) {
            return src_elem.is_assignable_to(dst_elem);
        }

        if let (Type::GenericInstance(cls1, args1), Type::GenericInstance(cls2, args2)) =
            (self, target)
        {
            if cls1 == cls2 && args1.len() == args2.len() {
                return args1
                    .iter()
                    .zip(args2.iter())
                    .all(|(a1, a2)| a1.is_assignable_to(a2));
            }
            return false;
        }

        if let (Type::GenericInstance(cls1, _), Type::Class(cls2)) = (self, target) {
            return cls1 == cls2;
        }

        if let (Type::Class(cls1), Type::GenericInstance(cls2, _)) = (self, target) {
            return cls1 == cls2;
        }

        if let (
            Type::Function {
                params: p1,
                return_type: r1,
            },
            Type::Function {
                params: p2,
                return_type: r2,
            },
        ) = (self, target)
        {
            if p1.len() != p2.len() {
                return false;
            }
            return p1.iter().zip(p2.iter()).all(|(a, b)| b.is_assignable_to(a))
                && r1.is_assignable_to(r2);
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
            Type::Array(elem) => write!(f, "{elem}[]"),
            Type::Class(name) => write!(f, "{name}"),
            Type::Interface(name) => write!(f, "{name}"),
            Type::GenericParam(name) => write!(f, "{name}"),
            Type::GenericInstance(name, args) => {
                let args_str = args
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{name}<{args_str}>")
            }
            Type::Function {
                params,
                return_type,
            } => {
                let params_str = params
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "func({params_str}): {return_type}")
            }
            Type::Unknown => write!(f, "<unknown>"),
        }
    }
}
