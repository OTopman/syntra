pub mod ast;
pub mod parser;
pub mod token;

pub use ast::*;
pub use parser::Parser;
pub use token::Token;

use aether_diagnostics::{Diagnostic, Span};
use logos::Logos;

pub fn tokenize(source: &str) -> (Vec<(Token, Span)>, Vec<Diagnostic>) {
    let mut lexer = Token::lexer(source);
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();

    while let Some(res) = lexer.next() {
        let span = Span::from(lexer.span());
        match res {
            Ok(token) => tokens.push((token, span)),
            Err(_) => {
                diagnostics.push(
                    Diagnostic::error("LANG-010", "Unrecognized character or token", span)
                        .with_label("Syntax error here"),
                );
            }
        }
    }

    (tokens, diagnostics)
}

pub fn parse(source: &str) -> Result<Program, Vec<Diagnostic>> {
    let (tokens, mut diagnostics) = tokenize(source);
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let end_span = Span::new(source.len(), source.len());
    let mut parser = Parser::new(&tokens, end_span);

    match parser.parse_program() {
        Ok(program) => Ok(program),
        Err(diag) => {
            diagnostics.push(diag);
            Err(diagnostics)
        }
    }
}
