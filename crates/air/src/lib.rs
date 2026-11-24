//! # Syntra AIR (Aether Intermediate Representation)
//!
//! Strongly typed SSA-form intermediate representation conforming to AIR-001 through AIR-032.
//! Features three-address instructions, basic block control flow graphs, loop-invariant detection,
//! and SSA dominance validation.

pub mod ir;
pub mod lowering;
pub mod print;
pub mod verifier;

pub use ir::*;
pub use lowering::Lowerer;
pub use print::print_air_module;
pub use verifier::AirVerifier;

// ─── SIR (Syntra Intermediate Representation) Architectural Aliases ───────────
// These type aliases provide architectural and specification alignment between
// the unified SSA Intermediate Representation (AIR) and Specification 03 (SIR).

pub type SirModule = AirModule;
pub type SirFunction = AirFunction;
pub type SirVerifier<'a> = AirVerifier<'a>;
pub type SirInstruction = Instruction;
pub type SirBasicBlock = BasicBlock;
pub type SirTerminator = Terminator;
pub type SirOp = Op;
pub use lowering::Lowerer as SirLowerer;
pub use print::print_air_module as print_sir_module;

use syntra_diagnostics::Diagnostic;
use syntra_syntax::ast::Program;

/// Lowers an AST program into SSA-form AIR and validates all basic block invariants.
///
/// # Arguments
/// * `program` - The parsed AST program root.
///
/// # Returns
/// * `Ok(AirModule)` - Verified SSA intermediate representation module.
/// * `Err(Vec<Diagnostic>)` - Verification failures or illegal SSA states.
pub fn lower_and_verify(program: &Program) -> Result<AirModule, Vec<Diagnostic>> {
    let mut lowerer = Lowerer::new();
    let module = lowerer.lower_program(program);

    let mut diagnostics = Vec::new();
    for func in &module.functions {
        let verifier = AirVerifier::new(func);
        if let Err(errs) = verifier.verify() {
            diagnostics.extend(errs);
        }
    }

    if diagnostics.is_empty() {
        Ok(module)
    } else {
        Err(diagnostics)
    }
}

/// Verifies SSA invariants across all functions within an existing AIR module.
///
/// Checks single-assignment validity, block termination, type agreement, and dominance.
///
/// # Arguments
/// * `module` - The AIR module to verify.
///
/// # Returns
/// List of diagnostics (empty if completely valid).
pub fn verify_module(module: &AirModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for func in &module.functions {
        let verifier = AirVerifier::new(func);
        if let Err(errs) = verifier.verify() {
            diagnostics.extend(errs);
        }
    }
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use syntra_diagnostics::Span;
    use syntra_frontend::Type;

    #[test]
    fn test_guard_verification() {
        let b0 = BlockId(0);
        let b_deopt = BlockId(1);
        let v_cond = ValueId(0);
        let v_dummy = ValueId(1);

        let block0 = BasicBlock {
            id: b0,
            params: Vec::new(),
            instructions: vec![
                Instruction {
                    result: v_cond,
                    ty: Type::Bool,
                    op: Op::ConstBool(true),
                    span: Span::default(),
                },
                Instruction {
                    result: v_dummy,
                    ty: Type::Void,
                    op: Op::Guard {
                        cond: v_cond,
                        deopt_target: b_deopt,
                        deopt_args: Vec::new(),
                        reason: "Speculative non-null invariant".to_string(),
                    },
                    span: Span::default(),
                },
            ],
            terminator: Terminator::Return(None, Span::default()),
        };

        let block1 = BasicBlock {
            id: b_deopt,
            params: Vec::new(),
            instructions: Vec::new(),
            terminator: Terminator::Return(None, Span::default()),
        };

        let func = AirFunction {
            name: "test_guard".to_string(),
            params: Vec::new(),
            return_type: Type::Void,
            blocks: vec![block0, block1],
            entry_block: b0,
        };

        let verifier = AirVerifier::new(&func);
        assert!(verifier.verify().is_ok());
    }

    #[test]
    fn test_sir_type_aliases_compatibility() {
        let b0 = BlockId(0);
        let block0 = SirBasicBlock {
            id: b0,
            params: Vec::new(),
            instructions: Vec::new(),
            terminator: SirTerminator::Return(None, Span::default()),
        };

        let func: SirFunction = SirFunction {
            name: "test_sir".to_string(),
            params: Vec::new(),
            return_type: Type::Void,
            blocks: vec![block0],
            entry_block: b0,
        };

        let verifier = SirVerifier::new(&func);
        assert!(verifier.verify().is_ok());

        let module: SirModule = SirModule {
            name: "sir_test_module".to_string(),
            functions: vec![func],
            class_sizes: std::collections::HashMap::new(),
        };

        let printed = print_sir_module(&module);
        assert!(printed.contains("sir_test_module"));
    }
}
