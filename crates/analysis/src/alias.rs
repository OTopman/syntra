use syntra_air::ir::{AirFunction, AirModule, Op, ValueId};

/// Relationship between two pointer / reference values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliasResult {
    /// Provably disjoint memory locations; a store to one cannot affect a load from the other
    NoAlias,
    /// Provably identical memory locations
    MustAlias,
    /// Memory locations may overlap or alias
    MayAlias,
}

/// The provenance origin of a pointer value
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PointerOrigin {
    LocalAlloc(usize), // Instruction index of Alloc/NewObject
    Field {
        base: ValueId,
        field_name: String,
        offset: usize,
    },
    Index {
        base: ValueId,
        index: ValueId,
    },
    Parameter(usize),
    CallResult(String),
    Unknown,
}

/// Intra- and inter-procedural alias analyzer
#[derive(Debug, Clone, Default)]
pub struct AliasReport {
    pub total_pointer_pairs_queried: usize,
    pub proven_no_alias: usize,
    pub must_alias: usize,
    pub may_alias: usize,
}

pub struct AliasAnalyzer;

impl AliasAnalyzer {
    pub fn query_alias(func: &AirFunction, v1: ValueId, v2: ValueId) -> AliasResult {
        if v1 == v2 {
            return AliasResult::MustAlias;
        }

        let origin1 = Self::trace_origin(func, v1);
        let origin2 = Self::trace_origin(func, v2);

        match (&origin1, &origin2) {
            (PointerOrigin::LocalAlloc(i1), PointerOrigin::LocalAlloc(i2)) => {
                if i1 != i2 {
                    AliasResult::NoAlias
                } else {
                    AliasResult::MustAlias
                }
            }
            (PointerOrigin::LocalAlloc(_), PointerOrigin::Parameter(_))
            | (PointerOrigin::Parameter(_), PointerOrigin::LocalAlloc(_)) => {
                // A newly allocated local object cannot alias a parameter upon creation
                AliasResult::NoAlias
            }
            (
                PointerOrigin::Field {
                    base: b1,
                    field_name: f1,
                    offset: o1,
                },
                PointerOrigin::Field {
                    base: b2,
                    field_name: f2,
                    offset: o2,
                },
            ) => {
                if b1 == b2 {
                    if f1 != f2 || o1 != o2 {
                        AliasResult::NoAlias
                    } else {
                        AliasResult::MustAlias
                    }
                } else if Self::query_alias(func, *b1, *b2) == AliasResult::NoAlias {
                    AliasResult::NoAlias
                } else {
                    AliasResult::MayAlias
                }
            }
            _ => AliasResult::MayAlias,
        }
    }

    pub fn trace_origin(func: &AirFunction, val: ValueId) -> PointerOrigin {
        for block in &func.blocks {
            for (idx, inst) in block.instructions.iter().enumerate() {
                if inst.result == val {
                    return match &inst.op {
                        Op::Alloc { .. } | Op::AllocArray { .. } | Op::NewObject { .. } => {
                            PointerOrigin::LocalAlloc(idx)
                        }
                        Op::GetField {
                            object,
                            field_name,
                            offset,
                        } => PointerOrigin::Field {
                            base: *object,
                            field_name: field_name.clone(),
                            offset: *offset,
                        },
                        Op::GetIndex { array, index } => PointerOrigin::Index {
                            base: *array,
                            index: *index,
                        },
                        Op::Call { func_name, .. } => PointerOrigin::CallResult(func_name.clone()),
                        _ => PointerOrigin::Unknown,
                    };
                }
            }
        }

        // Check if value is a parameter
        for (i, p) in func.params.iter().enumerate() {
            if p.id == val {
                return PointerOrigin::Parameter(i);
            }
        }

        PointerOrigin::Unknown
    }

    pub fn analyze_module(module: &AirModule) -> AliasReport {
        let mut report = AliasReport::default();

        for func in &module.functions {
            let mut pointers = Vec::new();
            for block in &func.blocks {
                for inst in &block.instructions {
                    match &inst.op {
                        Op::Alloc { .. }
                        | Op::AllocArray { .. }
                        | Op::NewObject { .. }
                        | Op::GetField { .. } => {
                            pointers.push(inst.result);
                        }
                        _ => {}
                    }
                }
            }

            for i in 0..pointers.len() {
                for j in (i + 1)..pointers.len() {
                    report.total_pointer_pairs_queried += 1;
                    match Self::query_alias(func, pointers[i], pointers[j]) {
                        AliasResult::NoAlias => report.proven_no_alias += 1,
                        AliasResult::MustAlias => report.must_alias += 1,
                        AliasResult::MayAlias => report.may_alias += 1,
                    }
                }
            }
        }

        report
    }
}
