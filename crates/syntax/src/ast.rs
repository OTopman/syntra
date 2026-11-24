use syntra_diagnostics::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Function(FunctionDecl),
    Class(ClassDecl),
    Interface(InterfaceDecl),
    Namespace(NamespaceDecl),
    Import(ImportDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamespaceDecl {
    pub name: String,
    pub doc: Option<String>,
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl {
    pub path: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub is_async: bool,
    pub name: String,
    pub generic_params: Vec<String>,
    pub doc: Option<String>,
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
    pub generic_args: Vec<TypeRef>,
    pub is_array: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub name: String,
    pub generic_params: Vec<String>,
    pub doc: Option<String>,
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
    pub doc: Option<String>,
    pub ty: TypeRef,
    pub name: String,
    pub init: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstructorDecl {
    pub doc: Option<String>,
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
    pub doc: Option<String>,
    pub methods: Vec<FunctionSig>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSig {
    pub is_async: bool,
    pub name: String,
    pub generic_params: Vec<String>,
    pub doc: Option<String>,
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
    For {
        init: Option<Box<Stmt>>,
        condition: Option<Expr>,
        update: Option<Box<Stmt>>,
        body: Block,
        span: Span,
    },
    ForIn {
        item_name: String,
        iterable: Expr,
        body: Block,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    TryCatch {
        try_block: Block,
        catch_param: Option<String>,
        catch_block: Option<Block>,
        finally_block: Option<Block>,
        span: Span,
    },
    Throw {
        expr: Expr,
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
        generic_args: Vec<TypeRef>,
        args: Vec<Expr>,
        span: Span,
    },
    ArrayLit {
        elements: Vec<Expr>,
        span: Span,
    },
    Index {
        array: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    Lambda {
        params: Vec<Param>,
        body_expr: Option<Box<Expr>>,
        body_block: Option<Block>,
        span: Span,
    },
    Await {
        expr: Box<Expr>,
        span: Span,
    },
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::VarDecl { span, .. } => *span,
            Stmt::Assignment { span, .. } => *span,
            Stmt::If { span, .. } => *span,
            Stmt::While { span, .. } => *span,
            Stmt::For { span, .. } => *span,
            Stmt::ForIn { span, .. } => *span,
            Stmt::Return { span, .. } => *span,
            Stmt::TryCatch { span, .. } => *span,
            Stmt::Throw { span, .. } => *span,
            Stmt::Expr { span, .. } => *span,
        }
    }
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
            Expr::ArrayLit { span, .. } => *span,
            Expr::Index { span, .. } => *span,
            Expr::Lambda { span, .. } => *span,
            Expr::Await { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    EqualEqual,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    Shl,
    Shr,
}
