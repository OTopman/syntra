pub mod eval;
pub mod value;

pub use eval::{Interpreter, RuntimeError};
pub use value::{Instance, Value};

use aether_syntax::ast::Program;

pub fn run(program: &Program) -> Result<Value, RuntimeError> {
    let mut interp = Interpreter::new();
    interp.run_program(program)
}
