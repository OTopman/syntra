use aether_diagnostics::Span;
use aether_frontend::Type;
use aether_syntax::ast::BinaryOp;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueId(pub usize);

impl fmt::Display for ValueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

/// Aether Memory Domain classification (Specification 02)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryDomain {
    Stack,
    Arena,
    ReferenceCounted,
    TracingManaged,
    Static,
}

impl fmt::Display for MemoryDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryDomain::Stack => write!(f, "stack"),
            MemoryDomain::Arena => write!(f, "arena"),
            MemoryDomain::ReferenceCounted => write!(f, "rc"),
            MemoryDomain::TracingManaged => write!(f, "gc"),
            MemoryDomain::Static => write!(f, "static"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AirModule {
    pub name: String,
    pub functions: Vec<AirFunction>,
}

#[derive(Debug, Clone)]
pub struct AirFunction {
    pub name: String,
    pub params: Vec<AirParam>,
    pub return_type: Type,
    pub blocks: Vec<BasicBlock>,
    pub entry_block: BlockId,
}

#[derive(Debug, Clone)]
pub struct AirParam {
    pub id: ValueId,
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockId,
    pub params: Vec<(ValueId, Type)>, // Block arguments for SSA phi elimination
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub result: ValueId,
    pub ty: Type,
    pub op: Op,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Op {
    ConstInt(i64),
    ConstFloat(f64),
    ConstBool(bool),
    ConstString(String),
    Binary {
        op: BinaryOp,
        left: ValueId,
        right: ValueId,
    },
    Alloc {
        domain: MemoryDomain,
        ty: Type,
    },
    Load {
        ptr: ValueId,
    },
    Store {
        ptr: ValueId,
        val: ValueId,
    },
    Call {
        func_name: String,
        args: Vec<ValueId>,
    },
    NewObject {
        class_name: String,
        domain: MemoryDomain,
        args: Vec<ValueId>,
    },
}

#[derive(Debug, Clone)]
pub enum Terminator {
    Return(Option<ValueId>, Span),
    Branch(BlockId, Vec<ValueId>, Span),
    CondBranch {
        cond: ValueId,
        then_block: BlockId,
        then_args: Vec<ValueId>,
        else_block: BlockId,
        else_args: Vec<ValueId>,
        span: Span,
    },
}
