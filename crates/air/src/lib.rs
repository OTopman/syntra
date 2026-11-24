pub mod ir;
pub mod lowering;
pub mod print;
pub mod verifier;

pub use ir::*;
pub use lowering::Lowerer;
pub use print::print_air_module;
pub use verifier::AirVerifier;

use aether_diagnostics::Diagnostic;
use aether_syntax::ast::Program;

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
