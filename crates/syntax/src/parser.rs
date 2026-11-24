use crate::ast::*;
use crate::token::Token;
use aether_diagnostics::{Diagnostic, Span};

pub struct Parser<'a> {
    tokens: &'a [(Token, Span)],
    cursor: usize,
    file_end_span: Span,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [(Token, Span)], file_end_span: Span) -> Self {
        Self {
            tokens,
            cursor: 0,
            file_end_span,
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, Diagnostic> {
        let mut items = Vec::new();
        while !self.is_at_end() {
            items.push(self.parse_item()?);
        }
        Ok(Program { items })
    }

    fn parse_item(&mut self) -> Result<Item, Diagnostic> {
        if self.check(&Token::Func) {
            Ok(Item::Function(self.parse_function()?))
        } else if self.check(&Token::Class) {
            Ok(Item::Class(self.parse_class()?))
        } else if self.check(&Token::Interface) {
            Ok(Item::Interface(self.parse_interface()?))
        } else {
            let span = self.peek_span();
            Err(Diagnostic::error(
                "LANG-010",
                format!("Expected top-level declaration ('class', 'interface', or 'func'), found {:?}", self.peek_token()),
                span,
            ))
        }
    }

    // --- Interface Parsing ---
    fn parse_interface(&mut self) -> Result<InterfaceDecl, Diagnostic> {
        let iface_span = self.expect(&Token::Interface, "Expected 'interface'")?;
        let (name, _) = self.expect_ident("Expected interface name")?;
        self.expect(&Token::LBrace, "Expected '{' after interface name")?;

        let mut methods = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            self.expect(&Token::Func, "Expected 'func' for interface method")?;
            let (m_name, _) = self.expect_ident("Expected method name")?;
            self.expect(&Token::LParen, "Expected '(' after method name")?;
            let params = self.parse_param_list()?;
            self.expect(&Token::RParen, "Expected ')' after parameter list")?;

            let return_type = if self.match_token(&Token::Colon) {
                Some(self.parse_type()?)
            } else {
                None
            };
            self.expect(&Token::Semicolon, "Expected ';' after interface method signature")?;

            methods.push(FunctionSig {
                name: m_name,
                params,
                return_type,
                span: self.prev_span(),
            });
        }

        let rbrace_span = self.expect(&Token::RBrace, "Expected '}' to close interface")?;
        Ok(InterfaceDecl {
            name,
            methods,
            span: iface_span.merge(rbrace_span),
        })
    }

    // --- Class Parsing ---
    fn parse_class(&mut self) -> Result<ClassDecl, Diagnostic> {
        let class_span = self.expect(&Token::Class, "Expected 'class'")?;
        let (name, _) = self.expect_ident("Expected class name")?;

        let extends = if self.match_token(&Token::Extends) {
            let (parent, _) = self.expect_ident("Expected parent class name after 'extends'")?;
            Some(parent)
        } else {
            None
        };

        let mut implements = Vec::new();
        if self.match_token(&Token::Implements) {
            loop {
                let (iface, _) = self.expect_ident("Expected interface name after 'implements'")?;
                implements.push(iface);
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
        }

        self.expect(&Token::LBrace, "Expected '{' after class declaration")?;

        let mut members = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            members.push(self.parse_class_member()?);
        }

        let rbrace_span = self.expect(&Token::RBrace, "Expected '}' to close class")?;
        Ok(ClassDecl {
            name,
            extends,
            implements,
            members,
            span: class_span.merge(rbrace_span),
        })
    }

    fn parse_class_member(&mut self) -> Result<ClassMember, Diagnostic> {
        let mut is_public = false;
        let mut is_static = false;
        let mut is_override = false;

        // Parse modifiers: public, private, static, override
        loop {
            if self.match_token(&Token::Public) {
                is_public = true;
            } else if self.match_token(&Token::Private) {
                is_public = false;
            } else if self.match_token(&Token::Static) {
                is_static = true;
            } else if self.match_token(&Token::Override) {
                is_override = true;
            } else {
                break;
            }
        }

        // Check if constructor: new(...) { ... }
        if self.check(&Token::New) {
            let new_span = self.expect(&Token::New, "Expected 'new'")?;
            self.expect(&Token::LParen, "Expected '(' for constructor parameters")?;
            let params = self.parse_param_list()?;
            self.expect(&Token::RParen, "Expected ')' after constructor parameters")?;
            let body = self.parse_block()?;
            let span = new_span.merge(body.span);
            return Ok(ClassMember::Constructor(ConstructorDecl { params, body, span }));
        }

        // Check if method: func name(...) ...
        if self.check(&Token::Func) {
            let fn_span = self.peek_span();
            let func = self.parse_function()?;
            let span = fn_span.merge(func.span);
            return Ok(ClassMember::Method(MethodDecl {
                is_public,
                is_static,
                is_override,
                func,
                span,
            }));
        }

        // Otherwise, it must be a field: Type name [= init];
        let ty = self.parse_type()?;
        let (field_name, field_span) = self.expect_ident("Expected field name")?;
        let init = if self.match_token(&Token::Assign) {
            Some(self.parse_expression(0)?)
        } else {
            None
        };
        let semi_span = self.expect(&Token::Semicolon, "Expected ';' after field declaration")?;

        Ok(ClassMember::Field(FieldDecl {
            is_public,
            is_static,
            ty,
            name: field_name,
            init,
            span: field_span.merge(semi_span),
        }))
    }

    // --- Function Parsing ---
    fn parse_function(&mut self) -> Result<FunctionDecl, Diagnostic> {
        let func_span = self.expect(&Token::Func, "Expected 'func'")?;
        let (name, _) = self.expect_ident("Expected function name")?;
        self.expect(&Token::LParen, "Expected '(' after function name")?;
        let params = self.parse_param_list()?;
        self.expect(&Token::RParen, "Expected ')' after parameters")?;

        let return_type = if self.match_token(&Token::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let body = self.parse_block()?;
        let span = func_span.merge(body.span);

        Ok(FunctionDecl {
            name,
            params,
            return_type,
            body,
            span,
        })
    }

    fn parse_param_list(&mut self) -> Result<Vec<Param>, Diagnostic> {
        let mut params = Vec::new();
        while !self.check(&Token::RParen) && !self.is_at_end() {
            let ty = self.parse_type()?;
            let (param_name, p_span) = self.expect_ident("Expected parameter name")?;
            params.push(Param {
                ty,
                name: param_name,
                span: p_span,
            });
            if !self.match_token(&Token::Comma) {
                break;
            }
        }
        Ok(params)
    }

    fn parse_type(&mut self) -> Result<TypeRef, Diagnostic> {
        let span = self.peek_span();
        let name = match self.peek_token() {
            Some(Token::IntType) => { self.advance(); "int".to_string() }
            Some(Token::FloatType) => { self.advance(); "float".to_string() }
            Some(Token::BoolType) => { self.advance(); "bool".to_string() }
            Some(Token::StringType) => { self.advance(); "string".to_string() }
            Some(Token::VoidType) => { self.advance(); "void".to_string() }
            Some(Token::Ident(custom)) => {
                let c = custom.clone();
                self.advance();
                c
            }
            _ => {
                return Err(Diagnostic::error(
                    "LANG-010",
                    format!("Expected type name (int, float, bool, string, void, or custom class), found {:?}", self.peek_token()),
                    span,
                ));
            }
        };

        let mut is_array = false;
        if self.match_token(&Token::LBracket) {
            self.expect(&Token::RBracket, "Expected ']' for array type")?;
            is_array = true;
        }

        Ok(TypeRef {
            name,
            is_array,
            span,
        })
    }

    fn parse_block(&mut self) -> Result<Block, Diagnostic> {
        let lbrace_span = self.expect(&Token::LBrace, "Expected '{'")?;
        let mut statements = Vec::new();

        while !self.check(&Token::RBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }

        let rbrace_span = self.expect(&Token::RBrace, "Expected '}'")?;
        Ok(Block {
            statements,
            span: lbrace_span.merge(rbrace_span),
        })
    }

    // --- Statement Parsing ---
    fn parse_statement(&mut self) -> Result<Stmt, Diagnostic> {
        // 1. Const variable declaration: const int MAX = 100;
        if self.match_token(&Token::Const) {
            let ty = self.parse_type()?;
            let (name, name_span) = self.expect_ident("Expected variable name after 'const'")?;
            self.expect(&Token::Assign, "Expected '=' in variable declaration")?;
            let init = self.parse_expression(0)?;
            let semi_span = self.expect(&Token::Semicolon, "Expected ';' after variable declaration")?;
            return Ok(Stmt::VarDecl {
                is_const: true,
                ty,
                name,
                init,
                span: name_span.merge(semi_span),
            });
        }

        // 2. Return statement
        if self.match_token(&Token::Return) {
            let ret_span = self.prev_span();
            let value = if !self.check(&Token::Semicolon) {
                Some(self.parse_expression(0)?)
            } else {
                None
            };
            let semi_span = self.expect(&Token::Semicolon, "Expected ';' after return")?;
            return Ok(Stmt::Return {
                value,
                span: ret_span.merge(semi_span),
            });
        }

        // 3. If statement
        if self.match_token(&Token::If) {
            let if_span = self.prev_span();
            let condition = self.parse_expression(0)?;
            let then_branch = self.parse_block()?;
            let else_branch = if self.match_token(&Token::Else) {
                if self.check(&Token::If) {
                    // else if ...
                    let nested_if = self.parse_statement()?;
                    Some(Block {
                        statements: vec![nested_if],
                        span: self.prev_span(),
                    })
                } else {
                    Some(self.parse_block()?)
                }
            } else {
                None
            };
            let span = if_span.merge(self.prev_span());
            return Ok(Stmt::If {
                condition,
                then_branch,
                else_branch,
                span,
            });
        }

        // 4. While loop
        if self.match_token(&Token::While) {
            let while_span = self.prev_span();
            let condition = self.parse_expression(0)?;
            let body = self.parse_block()?;
            let span = while_span.merge(body.span);
            return Ok(Stmt::While {
                condition,
                body,
                span,
            });
        }

        // 5. Check if it's a typed variable declaration: e.g. "int x = 10;" or "Player p = ...;"
        if self.is_start_of_type_decl() {
            let ty = self.parse_type()?;
            let (name, name_span) = self.expect_ident("Expected variable name")?;
            self.expect(&Token::Assign, "Expected '=' in variable declaration")?;
            let init = self.parse_expression(0)?;
            let semi_span = self.expect(&Token::Semicolon, "Expected ';' after variable declaration")?;
            return Ok(Stmt::VarDecl {
                is_const: false,
                ty,
                name,
                init,
                span: name_span.merge(semi_span),
            });
        }

        // 6. Otherwise, it is an Expression or Assignment statement:
        let expr = self.parse_expression(0)?;
        if self.match_token(&Token::Assign) {
            let val = self.parse_expression(0)?;
            let semi_span = self.expect(&Token::Semicolon, "Expected ';' after assignment")?;
            let span = expr.span().merge(semi_span);
            return Ok(Stmt::Assignment {
                target: expr,
                value: val,
                span,
            });
        }

        let semi_span = self.expect(&Token::Semicolon, "Expected ';' after expression")?;
        let span = expr.span().merge(semi_span);
        Ok(Stmt::Expr { expr, span })
    }

    fn is_start_of_type_decl(&self) -> bool {
        match self.peek_token() {
            Some(Token::IntType) | Some(Token::FloatType) | Some(Token::BoolType) | Some(Token::StringType) => true,
            Some(Token::Ident(_)) => {
                // If it's an Identifier followed by another Identifier or '[' (array), it's a custom type decl: "Player hero" or "int[] items"
                if let Some((_, _)) = self.tokens.get(self.cursor + 1) {
                    if let Some(next_tok) = self.tokens.get(self.cursor + 1).map(|(t, _)| t) {
                        return matches!(next_tok, Token::Ident(_) | Token::LBracket);
                    }
                }
                false
            }
            _ => false,
        }
    }

    // --- Pratt Parser for Expressions ---
    fn parse_expression(&mut self, min_bp: u8) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_primary()?;

        loop {
            // Check for member access / method call: object.member or object.method(args)
            if self.match_token(&Token::Dot) {
                let (member, m_span) = self.expect_ident("Expected property or method name after '.'")?;
                if self.match_token(&Token::LParen) {
                    let args = self.parse_arg_list()?;
                    let rparen_span = self.expect(&Token::RParen, "Expected ')' after method arguments")?;
                    let span = left.span().merge(rparen_span);
                    left = Expr::Call {
                        callee: Box::new(Expr::MemberAccess {
                            object: Box::new(left),
                            member,
                            span: m_span,
                        }),
                        args,
                        span,
                    };
                } else {
                    let span = left.span().merge(m_span);
                    left = Expr::MemberAccess {
                        object: Box::new(left),
                        member,
                        span,
                    };
                }
                continue;
            }

            // Check for direct function call: foo(a, b)
            if self.match_token(&Token::LParen) {
                let args = self.parse_arg_list()?;
                let rparen_span = self.expect(&Token::RParen, "Expected ')' after function arguments")?;
                let span = left.span().merge(rparen_span);
                left = Expr::Call {
                    callee: Box::new(left),
                    args,
                    span,
                };
                continue;
            }

            // Check for binary operators
            let op = match self.peek_token() {
                Some(Token::Plus) => BinaryOp::Add,
                Some(Token::Minus) => BinaryOp::Sub,
                Some(Token::Star) => BinaryOp::Mul,
                Some(Token::Slash) => BinaryOp::Div,
                Some(Token::EqualEqual) => BinaryOp::EqualEqual,
                Some(Token::NotEqual) => BinaryOp::NotEqual,
                Some(Token::Less) => BinaryOp::Less,
                Some(Token::LessEqual) => BinaryOp::LessEqual,
                Some(Token::Greater) => BinaryOp::Greater,
                Some(Token::GreaterEqual) => BinaryOp::GreaterEqual,
                _ => break,
            };

            let (l_bp, r_bp) = self.infix_binding_power(op);
            if l_bp < min_bp {
                break;
            }

            self.advance(); // consume operator
            let right = self.parse_expression(r_bp)?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn infix_binding_power(&self, op: BinaryOp) -> (u8, u8) {
        match op {
            BinaryOp::EqualEqual | BinaryOp::NotEqual => (1, 2),
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => (3, 4),
            BinaryOp::Add | BinaryOp::Sub => (5, 6),
            BinaryOp::Mul | BinaryOp::Div => (7, 8),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, Diagnostic> {
        let span = self.peek_span();
        match self.peek_token() {
            Some(Token::IntegerLit(val)) => {
                let val = *val;
                self.advance();
                Ok(Expr::Integer(val, span))
            }
            Some(Token::FloatLit(val)) => {
                let val = *val;
                self.advance();
                Ok(Expr::Float(val, span))
            }
            Some(Token::StringLit(val)) => {
                let s = val.clone();
                self.advance();
                Ok(Expr::StringLit(s, span))
            }
            Some(Token::True) => {
                self.advance();
                Ok(Expr::BoolLit(true, span))
            }
            Some(Token::False) => {
                self.advance();
                Ok(Expr::BoolLit(false, span))
            }
            Some(Token::This) => {
                self.advance();
                Ok(Expr::This(span))
            }
            Some(Token::New) => {
                self.advance(); // consume 'new'
                let (class_name, _) = self.expect_ident("Expected class name after 'new'")?;
                self.expect(&Token::LParen, "Expected '(' for constructor arguments")?;
                let args = self.parse_arg_list()?;
                let rparen_span = self.expect(&Token::RParen, "Expected ')' after constructor arguments")?;
                Ok(Expr::New {
                    class_name,
                    args,
                    span: span.merge(rparen_span),
                })
            }
            Some(Token::Ident(name)) => {
                let name = name.clone();
                self.advance();
                Ok(Expr::Variable(name, span))
            }
            Some(Token::LParen) => {
                self.advance();
                let expr = self.parse_expression(0)?;
                self.expect(&Token::RParen, "Expected ')'")?;
                Ok(expr)
            }
            _ => Err(Diagnostic::error(
                "LANG-010",
                format!("Expected expression, found {:?}", self.peek_token()),
                span,
            )),
        }
    }

    fn parse_arg_list(&mut self) -> Result<Vec<Expr>, Diagnostic> {
        let mut args = Vec::new();
        while !self.check(&Token::RParen) && !self.is_at_end() {
            args.push(self.parse_expression(0)?);
            if !self.match_token(&Token::Comma) {
                break;
            }
        }
        Ok(args)
    }

    // Helper functions
    fn check(&self, expected: &Token) -> bool {
        self.peek_token().map_or(false, |t| t == expected)
    }

    fn match_token(&mut self, expected: &Token) -> bool {
        if self.check(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: &Token, err_msg: &str) -> Result<Span, Diagnostic> {
        if self.check(expected) {
            let span = self.peek_span();
            self.advance();
            Ok(span)
        } else {
            Err(Diagnostic::error("LANG-010", err_msg, self.peek_span()))
        }
    }

    fn expect_ident(&mut self, err_msg: &str) -> Result<(String, Span), Diagnostic> {
        if let Some(Token::Ident(name)) = self.peek_token() {
            let name = name.clone();
            let span = self.peek_span();
            self.advance();
            Ok((name, span))
        } else {
            Err(Diagnostic::error("LANG-010", err_msg, self.peek_span()))
        }
    }

    fn peek_token(&self) -> Option<&Token> {
        self.tokens.get(self.cursor).map(|(t, _)| t)
    }

    fn peek_span(&self) -> Span {
        self.tokens.get(self.cursor).map(|(_, s)| *s).unwrap_or(self.file_end_span)
    }

    fn prev_span(&self) -> Span {
        if self.cursor > 0 {
            self.tokens[self.cursor - 1].1
        } else {
            Span::default()
        }
    }

    fn advance(&mut self) {
        if self.cursor < self.tokens.len() {
            self.cursor += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.cursor >= self.tokens.len()
    }
}
