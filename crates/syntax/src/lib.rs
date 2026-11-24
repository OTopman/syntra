pub mod ast;
pub mod parser;
pub mod token;

pub use ast::*;
pub use parser::Parser;
pub use token::Token;

use logos::Logos;
use syntra_diagnostics::{Diagnostic, Span};

pub fn tokenize(source: &str) -> (Vec<(Token, Span)>, Vec<Diagnostic>) {
    let mut lexer = Token::lexer(source);
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();

    while let Some(res) = lexer.next() {
        let span = Span::from(lexer.span());
        match res {
            Ok(Token::Comment) => {} // Skip regular trivia comments
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_arrays_and_loops() {
        let src = r#"
        func main(): int {
            int[] nums = [10, 20, 30];
            nums[1] = 99;
            int total = 0;
            for n in nums {
                total = total + n;
            }
            return total;
        }
        "#;
        let prog = parse(src).expect("Should parse array program");
        assert_eq!(prog.items.len(), 1);
    }

    #[test]
    fn test_parse_control_flow_variants() {
        let src = r#"
        func test(int x): int {
            if x > 10 {
                return 1;
            }
            if (x < 5) {
                return 2;
            }
            return 0;
        }
        "#;
        let prog = parse(src).expect("Should parse both if styles");
        assert_eq!(prog.items.len(), 1);
    }

    #[test]
    fn test_parse_doc_comments_and_namespace() {
        let src = r#"
        /// Math utilities namespace
        namespace Util {
            /// Calculates double of x
            func double_it(int x): int {
                return x * 2;
            }
        }
        "#;
        let prog = parse(src).expect("Should parse namespace with docs");
        assert_eq!(prog.items.len(), 1);
        if let Item::Namespace(ns) = &prog.items[0] {
            assert_eq!(ns.name, "Util");
            assert_eq!(ns.doc.as_deref(), Some("Math utilities namespace"));
            if let Item::Function(f) = &ns.items[0] {
                assert_eq!(f.name, "double_it");
                assert_eq!(f.doc.as_deref(), Some("Calculates double of x"));
            } else {
                panic!("Expected function inside namespace");
            }
        } else {
            panic!("Expected namespace item");
        }
    }

    #[test]
    fn test_parse_try_catch_finally_and_interpolation() {
        let src = r#"
        func main(): int {
            string name = "Syntra";
            string msg = $"Welcome to {name}! Math: {1 + 2}";
            try {
                throw "error message";
            } catch (err) {
                println(err);
            } finally {
                println("cleanup");
            }
            return 0;
        }
        "#;
        let prog = parse(src).expect("Should parse try/catch/finally and interpolation");
        assert_eq!(prog.items.len(), 1);
    }

    #[test]
    fn test_parse_generics_and_lambdas() {
        let src = r#"
        class Box<T> {
            public T val;
            new(T v) { this.val = v; }
            public func get(): T { return this.val; }
        }

        func main(): int {
            Box<int> b = new Box<int>(42);
            int[] nums = [1, 2, 3];
            int[] d = nums.map((int x) => x * 2);
            int[] e = nums.filter(x => x > 1);
            int s = nums.reduce((int acc, int x) => acc + x, 0);
            return b.get();
        }
        "#;
        let prog = parse(src).expect("Should parse generics and lambdas");
        assert_eq!(prog.items.len(), 2);
    }

    #[test]
    fn test_parse_async_await() {
        let src = r#"
        async fn fetch_user(int id): string {
            return "User#" + id;
        }

        class Service {
            public async func process(int x): int {
                return x * 10;
            }
        }

        fn main(): void {
            string user = await fetch_user(42);
            Service svc = new Service();
            int res = await svc.process(5);
        }
        "#;
        let prog = parse(src).expect("Should parse async fn and await expression");
        assert_eq!(prog.items.len(), 3);
        if let ast::Item::Function(f) = &prog.items[0] {
            assert!(f.is_async);
            assert_eq!(f.name, "fetch_user");
        } else {
            panic!("Expected async function");
        }
    }

    #[test]
    fn test_parse_await_precedence() {
        let src = r#"
        fn main(): void {
            int total = await a + await b;
        }
        "#;
        let prog = parse(src).expect("Should parse");
        if let ast::Item::Function(f) = &prog.items[0] {
            if let ast::Stmt::VarDecl { init, .. } = &f.body.statements[0] {
                match init {
                    ast::Expr::Binary {
                        op, left, right, ..
                    } => {
                        assert_eq!(*op, ast::BinaryOp::Add);
                        assert!(matches!(**left, ast::Expr::Await { .. }));
                        assert!(matches!(**right, ast::Expr::Await { .. }));
                    }
                    other => panic!("Expected BinaryOp::Add, got {other:?}"),
                }
            } else {
                panic!("Expected VarDecl");
            }
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn test_parse_custom_dto() {
        let src = r#"
        custom User {
            name: string
            age: int
        }

        struct Product {
            title: string;
            price: float;
        }

        class Standard {
            public string desc;
        }
        "#;
        let prog = parse(src).expect("Should parse custom and struct types with flexible fields");
        assert_eq!(prog.items.len(), 3);
        if let ast::Item::Class(u) = &prog.items[0] {
            assert_eq!(u.name, "User");
            assert_eq!(u.members.len(), 2);
            if let ast::ClassMember::Field(f) = &u.members[0] {
                assert_eq!(f.name, "name");
                assert_eq!(f.ty.name, "string");
                assert!(f.is_public);
            }
            if let ast::ClassMember::Field(f) = &u.members[1] {
                assert_eq!(f.name, "age");
                assert_eq!(f.ty.name, "int");
                assert!(f.is_public);
            }
        } else {
            panic!("Expected ClassDecl for User");
        }
    }

    #[test]
    fn test_parse_stdlib_http() {
        let http_src = std::fs::read_to_string("../../stdlib/http.syn").expect("read http.syn");
        let prog = parse(&http_src).expect("Should parse http.syn");
        assert!(!prog.items.is_empty());
    }
}
