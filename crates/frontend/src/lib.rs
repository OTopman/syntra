pub mod checker;
pub mod symbols;
pub mod types;

pub use checker::TypeChecker;
pub use symbols::*;
pub use types::Type;

use aether_diagnostics::Diagnostic;
use aether_syntax::ast::Program;

pub fn check(program: &Program) -> Result<(), Vec<Diagnostic>> {
    let checker = TypeChecker::new();
    checker.check_program(program)
}
