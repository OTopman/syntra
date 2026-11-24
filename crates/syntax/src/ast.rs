use aether_diagnostics::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Function(FunctionDecl),
    Class(ClassDecl),
    Interface(InterfaceDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeRef>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub ty: TypeRef,
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeRef {
    pub name: String,
    pub is_array: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub name: String,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub members: Vec<ClassMember>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClassMember {
    Field(FieldDecl),
    Constructor(ConstructorDecl),
    Method(MethodDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    pub is_public: bool,
    pub is_static: bool,
    pub ty: TypeRef,
    pub name: String,
    pub init: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstructorDecl {
    pub params: Vec<Param>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodDecl {
    pub is_public: bool,
    pub is_static: bool,
    pub is_override: bool,
    pub func: FunctionDecl,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDecl {
    pub name: String,
    pub methods: Vec<FunctionSig>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSig {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeRef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    VarDecl {
        is_const: bool,
        ty: TypeRef,
        name: String,
        init: Expr,
        span: Span,
    },
    Assignment {
        target: Expr,
        value: Expr,
        span: Span,
    },
    If {
        condition: Expr,
        then_branch: Block,
        else_branch: Option<Block>,
        span: Span,
    },
    While {
        condition: Expr,
        body: Block,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    Expr {
        expr: Expr,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Integer(i64, Span),
    Float(f64, Span),
    StringLit(String, Span),
    BoolLit(bool, Span),
    Variable(String, Span),
    This(Span),
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    MemberAccess {
        object: Box<Expr>,
        member: String,
        span: Span,
    },
    New {
        class_name: String,
        args: Vec<Expr>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Integer(_, s) => *s,
            Expr::Float(_, s) => *s,
            Expr::StringLit(_, s) => *s,
            Expr::BoolLit(_, s) => *s,
            Expr::Variable(_, s) => *s,
            Expr::This(s) => *s,
            Expr::Binary { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::MemberAccess { span, .. } => *span,
            Expr::New { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    EqualEqual,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}
