use crate::ast::*;
use crate::token::Token;
use syntra_diagnostics::{Diagnostic, Span};

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

    fn parse_doc_comments(&mut self) -> Option<String> {
        let mut lines = Vec::new();
        while let Some(Token::DocComment(line)) = self.peek_token() {
            lines.push(line.clone());
            self.advance();
        }
        if lines.is_empty() {
            None
        } else {
            Some(lines.join("\n"))
        }
    }

    fn parse_item(&mut self) -> Result<Item, Diagnostic> {
        let doc = self.parse_doc_comments();
        let is_async = self.match_token(&Token::Async);
        if self.check(&Token::Func) {
            Ok(Item::Function(self.parse_function(doc, is_async)?))
        } else if is_async {
            let span = self.peek_span();
            Err(Diagnostic::error(
                "LANG-010",
                format!(
                    "Expected 'fn' or 'func' after 'async', found {:?}",
                    self.peek_token()
                ),
                span,
            ))
        } else if self.check(&Token::Class)
            || self.check(&Token::Custom)
            || self.check(&Token::Struct)
        {
            Ok(Item::Class(self.parse_class(doc)?))
        } else if self.check(&Token::Interface) {
            Ok(Item::Interface(self.parse_interface(doc)?))
        } else if self.check(&Token::Namespace) {
            Ok(Item::Namespace(self.parse_namespace(doc)?))
        } else if self.check(&Token::Import) {
            Ok(Item::Import(self.parse_import()?))
        } else {
            let span = self.peek_span();
            Err(Diagnostic::error(
                "LANG-010",
                format!("Expected top-level declaration ('namespace', 'import', 'class', 'interface', or 'func'), found {:?}", self.peek_token()),
                span,
            ))
        }
    }

    // --- Namespace Parsing ---
    fn parse_namespace(&mut self, doc: Option<String>) -> Result<NamespaceDecl, Diagnostic> {
        let ns_span = self.expect(&Token::Namespace, "Expected 'namespace'")?;
        let (name, _) = self.expect_ident("Expected namespace name")?;
        self.expect(&Token::LBrace, "Expected '{' after namespace name")?;

        let mut items = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            items.push(self.parse_item()?);
        }

        let rbrace_span = self.expect(&Token::RBrace, "Expected '}' to close namespace")?;
        Ok(NamespaceDecl {
            name,
            doc,
            items,
            span: ns_span.merge(rbrace_span),
        })
    }

    // --- Import Parsing ---
    fn parse_import(&mut self) -> Result<ImportDecl, Diagnostic> {
        let import_span = self.expect(&Token::Import, "Expected 'import'")?;
        let path = if let Some(Token::StringLit(s)) = self.peek_token() {
            let s = s.clone();
            self.advance();
            s
        } else {
            let mut path_parts = Vec::new();
            let (first, _) = self.expect_ident("Expected module name after 'import'")?;
            path_parts.push(first);
            while self.match_token(&Token::Dot) {
                let (part, _) = self.expect_ident("Expected identifier after '.'")?;
                path_parts.push(part);
            }
            path_parts.join(".")
        };
        let semi_span = self.expect(&Token::Semicolon, "Expected ';' after import statement")?;
        Ok(ImportDecl {
            path,
            span: import_span.merge(semi_span),
        })
    }

    // --- Interface Parsing ---
    fn parse_interface(&mut self, doc: Option<String>) -> Result<InterfaceDecl, Diagnostic> {
        let iface_span = self.expect(&Token::Interface, "Expected 'interface'")?;
        let (name, _) = self.expect_ident("Expected interface name")?;
        self.expect(&Token::LBrace, "Expected '{' after interface name")?;

        let mut methods = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            let m_doc = self.parse_doc_comments();
            self.expect(&Token::Func, "Expected 'func' for interface method")?;
            let is_async = self.match_token(&Token::Async);
            let (m_name, _) = self.expect_ident("Expected method name")?;
            let mut generic_params = Vec::new();
            if self.match_token(&Token::Less) {
                loop {
                    let (gp, _) = self.expect_ident("Expected generic parameter name")?;
                    generic_params.push(gp);
                    if !self.match_token(&Token::Comma) {
                        break;
                    }
                }
                self.expect(&Token::Greater, "Expected '>' after generic parameters")?;
            }
            self.expect(&Token::LParen, "Expected '(' after method name")?;
            let params = self.parse_param_list()?;
            self.expect(&Token::RParen, "Expected ')' after parameter list")?;

            let return_type = if self.match_token(&Token::Colon) {
                Some(self.parse_type()?)
            } else {
                None
            };
            self.expect(
                &Token::Semicolon,
                "Expected ';' after interface method signature",
            )?;

            methods.push(FunctionSig {
                is_async,
                name: m_name,
                generic_params,
                doc: m_doc,
                params,
                return_type,
                span: self.prev_span(),
            });
        }

        let rbrace_span = self.expect(&Token::RBrace, "Expected '}' to close interface")?;
        Ok(InterfaceDecl {
            name,
            doc,
            methods,
            span: iface_span.merge(rbrace_span),
        })
    }

    // --- Class / Custom / Struct Parsing ---
    fn parse_class(&mut self, doc: Option<String>) -> Result<ClassDecl, Diagnostic> {
        let is_custom_or_struct = self.check(&Token::Custom) || self.check(&Token::Struct);
        let class_span = if self.match_token(&Token::Class)
            || self.match_token(&Token::Custom)
            || self.match_token(&Token::Struct)
        {
            self.prev_span()
        } else {
            return Err(Diagnostic::error(
                "LANG-010",
                "Expected 'class', 'custom', or 'struct'",
                self.peek_span(),
            ));
        };
        let (name, _) = self.expect_ident("Expected type name")?;

        let mut generic_params = Vec::new();
        if self.match_token(&Token::Less) {
            loop {
                let (gp, _) = self.expect_ident("Expected generic parameter name")?;
                generic_params.push(gp);
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
            self.expect(&Token::Greater, "Expected '>' after generic parameters")?;
        }

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

        self.expect(&Token::LBrace, "Expected '{' after type declaration")?;

        let mut members = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            members.push(self.parse_class_member(is_custom_or_struct)?);
        }

        let rbrace_span = self.expect(&Token::RBrace, "Expected '}' to close type declaration")?;
        Ok(ClassDecl {
            name,
            generic_params,
            doc,
            extends,
            implements,
            members,
            span: class_span.merge(rbrace_span),
        })
    }

    fn parse_class_member(&mut self, is_custom_or_struct: bool) -> Result<ClassMember, Diagnostic> {
        let doc = self.parse_doc_comments();
        let mut is_public = is_custom_or_struct;
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

        // Check if constructor: new(...) { ... } or constructor(...) { ... }
        let is_ctor = self.check(&Token::New)
            || (matches!(self.tokens.get(self.cursor).map(|(t, _)| t), Some(Token::Ident(id)) if id == "constructor")
                && self
                    .tokens
                    .get(self.cursor + 1)
                    .map(|(t, _)| t == &Token::LParen)
                    .unwrap_or(false));
        if is_ctor {
            let start_span = self.peek_span();
            self.advance();
            self.expect(&Token::LParen, "Expected '(' for constructor parameters")?;
            let params = self.parse_param_list()?;
            self.expect(&Token::RParen, "Expected ')' after constructor parameters")?;
            let body = self.parse_block()?;
            let span = start_span.merge(body.span);
            return Ok(ClassMember::Constructor(ConstructorDecl {
                doc,
                params,
                body,
                span,
            }));
        }

        // Check if method: [async] func name(...) ...
        let is_async = self.match_token(&Token::Async);
        if self.check(&Token::Func) {
            let fn_span = self.peek_span();
            let func = self.parse_function(doc, is_async)?;
            let span = fn_span.merge(func.span);
            return Ok(ClassMember::Method(MethodDecl {
                is_public,
                is_static,
                is_override,
                func,
                span,
            }));
        }

        // Otherwise, it must be a field:
        // Supports:
        // 1. `Type name [= init];`
        // 2. `name: Type [= init];` (with optional semicolon)
        let is_name_colon = matches!(
            self.tokens.get(self.cursor).map(|(t, _)| t),
            Some(Token::Ident(_))
        ) && self
            .tokens
            .get(self.cursor + 1)
            .map(|(t, _)| t == &Token::Colon)
            .unwrap_or(false);

        let (ty, field_name, field_span) = if is_name_colon {
            let (field_name, field_span) = self.expect_ident("Expected field name")?;
            self.expect(&Token::Colon, "Expected ':' after field name")?;
            let ty = self.parse_type()?;
            (ty, field_name, field_span)
        } else {
            let ty = self.parse_type()?;
            let (field_name, field_span) = self.expect_ident("Expected field name")?;
            (ty, field_name, field_span)
        };

        let init = if self.match_token(&Token::Assign) {
            Some(self.parse_expression(0)?)
        } else {
            None
        };
        let semi_span = if self.match_token(&Token::Semicolon) {
            self.prev_span()
        } else {
            ty.span
        };

        Ok(ClassMember::Field(FieldDecl {
            is_public,
            is_static,
            doc,
            ty,
            name: field_name,
            init,
            span: field_span.merge(semi_span),
        }))
    }

    // --- Function Parsing ---
    fn parse_function(
        &mut self,
        doc: Option<String>,
        is_async: bool,
    ) -> Result<FunctionDecl, Diagnostic> {
        let func_span = self.expect(&Token::Func, "Expected 'func'")?;
        let (name, _) = self.expect_ident("Expected function name")?;
        let mut generic_params = Vec::new();
        if self.match_token(&Token::Less) {
            loop {
                let (gp, _) = self.expect_ident("Expected generic parameter name")?;
                generic_params.push(gp);
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
            self.expect(&Token::Greater, "Expected '>' after generic parameters")?;
        }
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
            is_async,
            name,
            generic_params,
            doc,
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
            Some(Token::IntType) => {
                self.advance();
                "int".to_string()
            }
            Some(Token::FloatType) => {
                self.advance();
                "float".to_string()
            }
            Some(Token::BoolType) => {
                self.advance();
                "bool".to_string()
            }
            Some(Token::StringType) => {
                self.advance();
                "string".to_string()
            }
            Some(Token::VoidType) => {
                self.advance();
                "void".to_string()
            }
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

        let mut generic_args = Vec::new();
        if self.match_token(&Token::Less) {
            loop {
                generic_args.push(self.parse_type()?);
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
            self.expect(&Token::Greater, "Expected '>' after generic type arguments")?;
        }

        let mut is_array = false;
        if self.match_token(&Token::LBracket) {
            self.expect(&Token::RBracket, "Expected ']' for array type")?;
            is_array = true;
        }

        Ok(TypeRef {
            name,
            generic_args,
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
            let semi_span =
                self.expect(&Token::Semicolon, "Expected ';' after variable declaration")?;
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

        // Throw statement
        if self.match_token(&Token::Throw) {
            let throw_span = self.prev_span();
            let expr = self.parse_expression(0)?;
            let semi_span = self.expect(&Token::Semicolon, "Expected ';' after throw")?;
            return Ok(Stmt::Throw {
                expr,
                span: throw_span.merge(semi_span),
            });
        }

        // Try / Catch / Finally statement
        if self.match_token(&Token::Try) {
            let try_span = self.prev_span();
            let try_block = self.parse_block()?;

            let mut catch_param = None;
            let mut catch_block = None;
            if self.match_token(&Token::Catch) {
                if self.match_token(&Token::LParen) {
                    if self.is_start_of_type_decl() {
                        let _ty = self.parse_type()?;
                    }
                    let (p_name, _) =
                        self.expect_ident("Expected identifier for catch parameter")?;
                    catch_param = Some(p_name);
                    self.expect(&Token::RParen, "Expected ')' after catch parameter")?;
                }
                catch_block = Some(self.parse_block()?);
            }

            let mut finally_block = None;
            if self.match_token(&Token::Finally) {
                finally_block = Some(self.parse_block()?);
            }

            if catch_block.is_none() && finally_block.is_none() {
                return Err(Diagnostic::error(
                    "LANG-010",
                    "Expected 'catch' or 'finally' block after 'try'",
                    try_span,
                ));
            }

            let end_span = if let Some(ref fb) = finally_block {
                fb.span
            } else if let Some(ref cb) = catch_block {
                cb.span
            } else {
                try_block.span
            };

            return Ok(Stmt::TryCatch {
                try_block,
                catch_param,
                catch_block,
                finally_block,
                span: try_span.merge(end_span),
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

        // 5. For loop
        if self.match_token(&Token::For) {
            let for_span = self.prev_span();
            let has_paren = self.match_token(&Token::LParen);

            // Check if it's "for item in iterable" or "for (item in iterable)"
            let is_for_in = if let Some(Token::Ident(_)) = self.peek_token() {
                self.tokens
                    .get(self.cursor + 1)
                    .is_some_and(|(t, _)| *t == Token::In)
            } else {
                false
            };

            if is_for_in {
                let (item_name, _) = self.expect_ident("Expected identifier in for-in loop")?;
                self.expect(&Token::In, "Expected 'in' in for-in loop")?;
                let iterable = self.parse_expression(0)?;
                if has_paren {
                    self.expect(&Token::RParen, "Expected ')' after for-in loop")?;
                }
                let body = self.parse_block()?;
                let span = for_span.merge(body.span);
                return Ok(Stmt::ForIn {
                    item_name,
                    iterable,
                    body,
                    span,
                });
            }

            // Standard for: for [init]; [cond]; [update] { body }
            let init = if self.match_token(&Token::Semicolon) {
                None
            } else if self.is_start_of_type_decl() {
                let ty = self.parse_type()?;
                let (name, name_span) = self.expect_ident("Expected variable name")?;
                self.expect(&Token::Assign, "Expected '=' in variable declaration")?;
                let init_expr = self.parse_expression(0)?;
                let semi_span = self.expect(&Token::Semicolon, "Expected ';' after for init")?;
                Some(Box::new(Stmt::VarDecl {
                    is_const: false,
                    ty,
                    name,
                    init: init_expr,
                    span: name_span.merge(semi_span),
                }))
            } else {
                let expr = self.parse_expression(0)?;
                if self.match_token(&Token::Assign) {
                    let val = self.parse_expression(0)?;
                    let semi_span =
                        self.expect(&Token::Semicolon, "Expected ';' after assignment")?;
                    Some(Box::new(Stmt::Assignment {
                        target: expr,
                        value: val,
                        span: semi_span,
                    }))
                } else {
                    let semi_span =
                        self.expect(&Token::Semicolon, "Expected ';' after expression")?;
                    Some(Box::new(Stmt::Expr {
                        expr,
                        span: semi_span,
                    }))
                }
            };

            let condition = if self.check(&Token::Semicolon) {
                None
            } else {
                Some(self.parse_expression(0)?)
            };
            self.expect(&Token::Semicolon, "Expected ';' after for condition")?;

            let update = if (has_paren && self.check(&Token::RParen))
                || (!has_paren && self.check(&Token::LBrace))
            {
                None
            } else {
                let expr = self.parse_expression(0)?;
                if self.match_token(&Token::Assign) {
                    let val = self.parse_expression(0)?;
                    Some(Box::new(Stmt::Assignment {
                        target: expr,
                        value: val,
                        span: self.prev_span(),
                    }))
                } else {
                    Some(Box::new(Stmt::Expr {
                        expr,
                        span: self.prev_span(),
                    }))
                }
            };

            if has_paren {
                self.expect(&Token::RParen, "Expected ')' after for loop header")?;
            }

            let body = self.parse_block()?;
            let span = for_span.merge(body.span);
            return Ok(Stmt::For {
                init,
                condition,
                update,
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
            let semi_span =
                self.expect(&Token::Semicolon, "Expected ';' after variable declaration")?;
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
            Some(Token::IntType)
            | Some(Token::FloatType)
            | Some(Token::BoolType)
            | Some(Token::StringType) => true,
            Some(Token::Ident(_)) => {
                if let Some(next_tok) = self.tokens.get(self.cursor + 1).map(|(t, _)| t) {
                    if matches!(next_tok, Token::Ident(_)) {
                        return true;
                    }
                    if *next_tok == Token::LBracket {
                        if let Some(after_bracket) =
                            self.tokens.get(self.cursor + 2).map(|(t, _)| t)
                        {
                            return *after_bracket == Token::RBracket;
                        }
                    }
                    if *next_tok == Token::Less {
                        let mut depth = 1;
                        let mut idx = self.cursor + 2;
                        while idx < self.tokens.len() && depth > 0 {
                            if self.tokens[idx].0 == Token::Less {
                                depth += 1;
                            } else if self.tokens[idx].0 == Token::Greater {
                                depth -= 1;
                            }
                            idx += 1;
                        }
                        if depth == 0 && idx < self.tokens.len() {
                            return matches!(self.tokens[idx].0, Token::Ident(_));
                        }
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
                let (member, m_span) =
                    self.expect_ident("Expected property or method name after '.'")?;
                if self.match_token(&Token::LParen) {
                    let args = self.parse_arg_list()?;
                    let rparen_span =
                        self.expect(&Token::RParen, "Expected ')' after method arguments")?;
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
                let rparen_span =
                    self.expect(&Token::RParen, "Expected ')' after function arguments")?;
                let span = left.span().merge(rparen_span);
                left = Expr::Call {
                    callee: Box::new(left),
                    args,
                    span,
                };
                continue;
            }

            // Check for index access: arr[i]
            if self.match_token(&Token::LBracket) {
                let index = self.parse_expression(0)?;
                let rbracket_span = self.expect(&Token::RBracket, "Expected ']' after index")?;
                let span = left.span().merge(rbracket_span);
                left = Expr::Index {
                    array: Box::new(left),
                    index: Box::new(index),
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
                Some(Token::Percent) => BinaryOp::Mod,
                Some(Token::Amp) => BinaryOp::BitwiseAnd,
                Some(Token::Pipe) => BinaryOp::BitwiseOr,
                Some(Token::Caret) => BinaryOp::BitwiseXor,
                Some(Token::LessLess) => BinaryOp::Shl,
                Some(Token::GreaterGreater) => BinaryOp::Shr,
                Some(Token::EqualEqual) => BinaryOp::EqualEqual,
                Some(Token::NotEqual) => BinaryOp::NotEqual,
                Some(Token::Less) => BinaryOp::Less,
                Some(Token::LessEqual) => BinaryOp::LessEqual,
                Some(Token::Greater) => BinaryOp::Greater,
                Some(Token::GreaterEqual) => BinaryOp::GreaterEqual,
                Some(Token::AmpAmp) => BinaryOp::And,
                Some(Token::PipePipe) => BinaryOp::Or,
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
            BinaryOp::Or => (1, 2),
            BinaryOp::And => (3, 4),
            BinaryOp::BitwiseOr => (5, 6),
            BinaryOp::BitwiseXor => (7, 8),
            BinaryOp::BitwiseAnd => (9, 10),
            BinaryOp::EqualEqual | BinaryOp::NotEqual => (11, 12),
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
                (13, 14)
            }
            BinaryOp::Shl | BinaryOp::Shr => (15, 16),
            BinaryOp::Add | BinaryOp::Sub => (17, 18),
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => (19, 20),
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
            Some(Token::InterpolatedStringLit(val)) => {
                let s = val.clone();
                self.advance();
                self.parse_interpolated_string(&s, span)
            }
            Some(Token::Await) => {
                let await_span = self.peek_span();
                self.advance(); // consume 'await'
                let expr = self.parse_expression(21)?;
                let span = await_span.merge(expr.span());
                Ok(Expr::Await {
                    expr: Box::new(expr),
                    span,
                })
            }
            Some(Token::Minus) => {
                let minus_span = self.peek_span();
                self.advance(); // consume '-'
                let expr = self.parse_primary()?;
                match expr {
                    Expr::Integer(val, span) => Ok(Expr::Integer(-val, minus_span.merge(span))),
                    Expr::Float(val, span) => Ok(Expr::Float(-val, minus_span.merge(span))),
                    other => {
                        let span = minus_span.merge(other.span());
                        Ok(Expr::Binary {
                            op: BinaryOp::Sub,
                            left: Box::new(Expr::Integer(0, minus_span)),
                            right: Box::new(other),
                            span,
                        })
                    }
                }
            }
            Some(Token::Tilde) => {
                let tilde_span = self.peek_span();
                self.advance(); // consume '~'
                let expr = self.parse_primary()?;
                let span = tilde_span.merge(expr.span());
                Ok(Expr::Binary {
                    op: BinaryOp::BitwiseXor,
                    left: Box::new(expr),
                    right: Box::new(Expr::Integer(-1, tilde_span)),
                    span,
                })
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
                let mut generic_args = Vec::new();
                if self.match_token(&Token::Less) {
                    loop {
                        generic_args.push(self.parse_type()?);
                        if !self.match_token(&Token::Comma) {
                            break;
                        }
                    }
                    self.expect(&Token::Greater, "Expected '>' after generic type arguments")?;
                }
                self.expect(&Token::LParen, "Expected '(' for constructor arguments")?;
                let args = self.parse_arg_list()?;
                let rparen_span =
                    self.expect(&Token::RParen, "Expected ')' after constructor arguments")?;
                Ok(Expr::New {
                    class_name,
                    generic_args,
                    args,
                    span: span.merge(rparen_span),
                })
            }
            Some(Token::Ident(name)) => {
                if let Some(next_tok) = self.tokens.get(self.cursor + 1).map(|(t, _)| t) {
                    if *next_tok == Token::FatArrow {
                        let name = name.clone();
                        let id_span = span;
                        self.advance(); // consume ident
                        self.advance(); // consume '=>'
                        let (body_expr, body_block) = if self.check(&Token::LBrace) {
                            (None, Some(self.parse_block()?))
                        } else {
                            (Some(Box::new(self.parse_expression(0)?)), None)
                        };
                        let end_span = body_expr
                            .as_ref()
                            .map(|e| e.span())
                            .unwrap_or_else(|| body_block.as_ref().unwrap().span);
                        return Ok(Expr::Lambda {
                            params: vec![Param {
                                ty: TypeRef {
                                    name: "any".to_string(),
                                    generic_args: Vec::new(),
                                    is_array: false,
                                    span: id_span,
                                },
                                name,
                                span: id_span,
                            }],
                            body_expr,
                            body_block,
                            span: id_span.merge(end_span),
                        });
                    }
                }
                let name = name.clone();
                self.advance();
                Ok(Expr::Variable(name, span))
            }
            Some(Token::LParen) => {
                let is_lambda = {
                    let mut depth = 1;
                    let mut idx = self.cursor + 1;
                    while idx < self.tokens.len() && depth > 0 {
                        if self.tokens[idx].0 == Token::LParen {
                            depth += 1;
                        } else if self.tokens[idx].0 == Token::RParen {
                            depth -= 1;
                        }
                        idx += 1;
                    }
                    depth == 0 && idx < self.tokens.len() && self.tokens[idx].0 == Token::FatArrow
                };

                if is_lambda {
                    let lparen_span = span;
                    self.advance(); // consume '('
                    let mut params = Vec::new();
                    if !self.check(&Token::RParen) {
                        loop {
                            if self.is_start_of_type_decl() {
                                let ty = self.parse_type()?;
                                let (p_name, p_span) =
                                    self.expect_ident("Expected parameter name")?;
                                params.push(Param {
                                    ty,
                                    name: p_name,
                                    span: p_span,
                                });
                            } else {
                                let (p_name, p_span) =
                                    self.expect_ident("Expected parameter name")?;
                                params.push(Param {
                                    ty: TypeRef {
                                        name: "any".to_string(),
                                        generic_args: Vec::new(),
                                        is_array: false,
                                        span: p_span,
                                    },
                                    name: p_name,
                                    span: p_span,
                                });
                            }
                            if !self.match_token(&Token::Comma) {
                                break;
                            }
                        }
                    }
                    self.expect(&Token::RParen, "Expected ')' after lambda parameters")?;
                    self.expect(&Token::FatArrow, "Expected '=>' in lambda")?;

                    let (body_expr, body_block) = if self.check(&Token::LBrace) {
                        (None, Some(self.parse_block()?))
                    } else {
                        (Some(Box::new(self.parse_expression(0)?)), None)
                    };
                    let end_span = body_expr
                        .as_ref()
                        .map(|e| e.span())
                        .unwrap_or_else(|| body_block.as_ref().unwrap().span);
                    return Ok(Expr::Lambda {
                        params,
                        body_expr,
                        body_block,
                        span: lparen_span.merge(end_span),
                    });
                }

                self.advance();
                let expr = self.parse_expression(0)?;
                self.expect(&Token::RParen, "Expected ')'")?;
                Ok(expr)
            }
            Some(Token::LBracket) => {
                let lbracket_span = self.peek_span();
                self.advance(); // consume '['
                let mut elements = Vec::new();
                if !self.check(&Token::RBracket) {
                    loop {
                        elements.push(self.parse_expression(0)?);
                        if !self.match_token(&Token::Comma) {
                            break;
                        }
                    }
                }
                let rbracket_span =
                    self.expect(&Token::RBracket, "Expected ']' at end of array literal")?;
                Ok(Expr::ArrayLit {
                    elements,
                    span: lbracket_span.merge(rbracket_span),
                })
            }
            _ => Err(Diagnostic::error(
                "LANG-010",
                format!("Expected expression, found {:?}", self.peek_token()),
                span,
            )),
        }
    }

    fn parse_interpolated_string(&mut self, raw: &str, span: Span) -> Result<Expr, Diagnostic> {
        let mut pieces: Vec<Expr> = Vec::new();
        let mut chars = raw.chars().peekable();
        let mut current_lit = String::new();

        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('{') => current_lit.push('{'),
                    Some('}') => current_lit.push('}'),
                    Some('n') => current_lit.push('\n'),
                    Some('t') => current_lit.push('\t'),
                    Some('r') => current_lit.push('\r'),
                    Some('\\') => current_lit.push('\\'),
                    Some('"') => current_lit.push('"'),
                    Some(other) => {
                        current_lit.push('\\');
                        current_lit.push(other);
                    }
                    None => current_lit.push('\\'),
                }
            } else if c == '{' {
                if !current_lit.is_empty() {
                    pieces.push(Expr::StringLit(current_lit.clone(), span));
                    current_lit.clear();
                }
                let mut expr_str = String::new();
                let mut depth = 1;
                while let Some(ec) = chars.next() {
                    if ec == '\\' {
                        match chars.next() {
                            Some('"') => expr_str.push('"'),
                            Some(escaped) => {
                                expr_str.push('\\');
                                expr_str.push(escaped);
                            }
                            None => expr_str.push('\\'),
                        }
                    } else if ec == '{' {
                        depth += 1;
                        expr_str.push('{');
                    } else if ec == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        expr_str.push('}');
                    } else {
                        expr_str.push(ec);
                    }
                }

                if depth != 0 {
                    return Err(Diagnostic::error(
                        "LANG-010",
                        "Unterminated '{' in interpolated string",
                        span,
                    ));
                }

                let (sub_tokens, sub_diags) = crate::tokenize(&expr_str);
                if !sub_diags.is_empty() {
                    return Err(sub_diags.into_iter().next().unwrap());
                }
                let mut sub_parser = Parser::new(&sub_tokens, span);
                let expr = sub_parser.parse_expression(0)?;
                pieces.push(expr);
            } else {
                current_lit.push(c);
            }
        }

        if !current_lit.is_empty() {
            pieces.push(Expr::StringLit(current_lit, span));
        }

        if pieces.is_empty() {
            return Ok(Expr::StringLit("".to_string(), span));
        }
        if pieces.len() == 1 {
            if let Expr::StringLit(..) = &pieces[0] {
                return Ok(pieces.remove(0));
            }
        }

        let mut iter = pieces.into_iter();
        let first = iter.next().unwrap();
        let mut acc = match first {
            Expr::StringLit(..) => first,
            other => Expr::Binary {
                op: BinaryOp::Add,
                left: Box::new(Expr::StringLit("".to_string(), span)),
                right: Box::new(other),
                span,
            },
        };

        for next in iter {
            acc = Expr::Binary {
                op: BinaryOp::Add,
                left: Box::new(acc),
                right: Box::new(next),
                span,
            };
        }

        Ok(acc)
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
        self.peek_token() == Some(expected)
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
        match self.peek_token() {
            Some(Token::Ident(name)) => {
                let name = name.clone();
                let span = self.peek_span();
                self.advance();
                Ok((name, span))
            }
            Some(Token::Await) => {
                let span = self.peek_span();
                self.advance();
                Ok(("await".to_string(), span))
            }
            _ => Err(Diagnostic::error("LANG-010", err_msg, self.peek_span())),
        }
    }

    fn peek_token(&self) -> Option<&Token> {
        self.tokens.get(self.cursor).map(|(t, _)| t)
    }

    fn peek_span(&self) -> Span {
        self.tokens
            .get(self.cursor)
            .map(|(_, s)| *s)
            .unwrap_or(self.file_end_span)
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
