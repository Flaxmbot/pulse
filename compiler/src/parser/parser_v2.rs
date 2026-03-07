use crate::ast::*;
use crate::lexer::{Lexer, Token};
use crate::types::{Type, TypedParam};
use pulse_ast::{Constant, PulseError, PulseResult};

pub struct ParserV2<'a> {
    source: &'a str,
    pub lexer: Lexer<'a>,
    pub current: Token,
    pub previous: Token,
    previous_line: usize,
}

impl<'a> ParserV2<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            lexer: Lexer::new(source),
            current: Token::Eof,
            previous: Token::Eof,
            previous_line: 1,
        }
    }

    pub fn parse(&mut self) -> PulseResult<Script> {
        self.advance()?;
        let mut declarations = Vec::new();
        while !self.is_at_end() {
            declarations.push(self.declaration()?);
        }
        Ok(Script { declarations })
    }

    pub fn advance(&mut self) -> PulseResult<()> {
        self.previous_line = self.lexer.token_line;
        self.previous = self.current.clone();
        self.current = self.lexer.next_token()?;
        Ok(())
    }

    fn check(&self, token_type: Token) -> bool {
        Self::tokens_match(&self.current, &token_type)
    }

    fn matches(&mut self, token_type: Token) -> PulseResult<bool> {
        if self.check(token_type) {
            self.advance()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn is_at_end(&self) -> bool {
        self.current == Token::Eof
    }

    pub fn consume(&mut self, expected: Token, message: &str) -> PulseResult<Token> {
        if self.check(expected.clone()) {
            let t = self.current.clone();
            self.advance()?;
            Ok(t)
        } else {
            Err(self.error_expected(message, &expected))
        }
    }

    pub fn line(&self) -> usize {
        self.current_line()
    }

    pub fn previous_line(&self) -> usize {
        self.previous_line
    }

    fn current_line(&self) -> usize {
        self.lexer.token_line.max(1)
    }

    fn current_column(&self) -> usize {
        self.lexer.token_column.max(1)
    }

    fn tokens_match(actual: &Token, expected: &Token) -> bool {
        match (actual, expected) {
            (Token::Identifier(_), Token::Identifier(_))
            | (Token::String(_), Token::String(_))
            | (Token::Int(_), Token::Int(_))
            | (Token::Float(_), Token::Float(_))
            | (Token::DocComment(_), Token::DocComment(_))
            | (Token::InterpolatedString(_), Token::InterpolatedString(_)) => true,
            _ => actual == expected,
        }
    }

    fn describe_token(&self, token: &Token) -> String {
        match token {
            Token::Identifier(name) => format!("identifier '{}'", name),
            Token::String(value) => format!("string \"{}\"", value),
            Token::Int(value) => format!("integer {}", value),
            Token::Float(value) => format!("float {}", value),
            Token::Eof => "end of file".to_string(),
            _ => format!("{:?}", token),
        }
    }

    fn render_source_excerpt(&self, line: usize, column: usize) -> Option<String> {
        let source_line = self.source.lines().nth(line.saturating_sub(1))?;
        if source_line.trim().is_empty() {
            return None;
        }

        let caret_col = column.saturating_sub(1);
        let mut pointer = String::new();
        pointer.push_str(&" ".repeat(caret_col));
        pointer.push('^');
        Some(format!("{source_line}\n{pointer}"))
    }

    pub fn error_at(&self, message: impl Into<String>, line: usize, column: usize) -> PulseError {
        let mut full = format!("{} (at {}:{})", message.into(), line, column);
        if let Some(excerpt) = self.render_source_excerpt(line, column) {
            full.push('\n');
            full.push_str(&excerpt);
        }
        PulseError::CompileError(full, line)
    }

    pub fn error(&self, message: impl Into<String>) -> PulseError {
        self.error_at(message, self.current_line(), self.current_column())
    }

    fn error_expected(&self, message: &str, expected: &Token) -> PulseError {
        let expected_desc = self.describe_token(expected);
        let found_desc = self.describe_token(&self.current);
        self.error(format!(
            "{} Found {} instead of {}.",
            message, found_desc, expected_desc
        ))
    }

    // --- Declarations ---

    fn declaration(&mut self) -> PulseResult<Decl> {
        if self.matches(Token::Import)? {
            self.import_declaration()
        } else if self.matches(Token::Class)? {
            self.class_declaration()
        } else if self.matches(Token::Actor)? {
            self.actor_declaration()
        } else if self.matches(Token::Shared)? {
            self.shared_memory_declaration()
        } else if self.matches(Token::Atomic)? {
            self.atomic_declaration()
        } else if self.matches(Token::Fence)?
            || self.matches(Token::Acquire)?
            || self.matches(Token::Release)?
        {
            Ok(Decl::Stmt(self.empty_unit_statement()?))
        } else if self.matches(Token::Let)? {
            self.var_declaration()
        } else if self.matches(Token::Const)? {
            self.const_declaration()
        } else if self.matches(Token::Fn)? || self.matches(Token::Def)? {
            self.function_declaration()
        } else {
            Ok(Decl::Stmt(self.statement()?))
        }
    }

    fn import_declaration(&mut self) -> PulseResult<Decl> {
        let path_token =
            self.consume(Token::String("".to_string()), "Expect module path string.")?;
        let path = if let Token::String(s) = path_token {
            s
        } else {
            unreachable!()
        };

        let alias = if self.matches(Token::As)? {
            let alias_token = self.consume_identifier("Expect alias name after 'as'.")?;
            if let Token::Identifier(s) = alias_token {
                Some(s)
            } else {
                None
            }
        } else {
            None
        };

        self.consume(Token::Semicolon, "Expect ';' after import.")?;
        Ok(Decl::Stmt(Stmt::Import(path, alias)))
    }

    fn class_declaration(&mut self) -> PulseResult<Decl> {
        let name_token = self.consume_identifier("Expect class name.")?;
        let name = if let Token::Identifier(s) = name_token {
            s
        } else {
            unreachable!()
        };

        let parent = if self.matches(Token::Extends)? {
            let parent_token = self.consume_identifier("Expect parent class name.")?;
            if let Token::Identifier(s) = parent_token {
                Some(s)
            } else {
                None
            }
        } else {
            None
        };

        self.consume(Token::LeftBrace, "Expect '{' before class body.")?;

        let mut methods = Vec::new();
        while !self.check(Token::RightBrace) && !self.is_at_end() {
            // Parse method (function declaration without fn keyword)
            let method = self.method_declaration()?;
            methods.push(method);
        }

        self.consume(Token::RightBrace, "Expect '}' after class body.")?;
        Ok(Decl::Class(name, parent, methods))
    }

    fn method_declaration(&mut self) -> PulseResult<Decl> {
        // Methods can start with 'fn' or just the name
        let _ = self.matches(Token::Fn)?;

        let name_token = self.consume_identifier("Expect method name.")?;
        let name = if let Token::Identifier(s) = name_token {
            s
        } else {
            unreachable!()
        };

        self.consume(Token::LeftParen, "Expect '(' after method name.")?;
        let mut params = Vec::new();

        // Add 'this' as implicit first parameter
        params.push(TypedParam {
            name: "this".to_string(),
            type_annotation: None,
        });

        if !self.check(Token::RightParen) {
            loop {
                let p_name_token = self.consume_identifier("Expect parameter name.")?;
                let p_name = if let Token::Identifier(s) = p_name_token {
                    s
                } else {
                    unreachable!()
                };

                let mut type_annotation = None;
                if self.matches(Token::Colon)? {
                    type_annotation = Some(self.parse_type()?);
                }

                params.push(TypedParam {
                    name: p_name,
                    type_annotation,
                });
                if !self.matches(Token::Comma)? {
                    break;
                }
            }
        }
        self.consume(Token::RightParen, "Expect ')' after parameters.")?;

        let mut return_type = None;
        if self.matches(Token::Arrow)? {
            return_type = Some(self.parse_type()?);
        }

        self.consume(Token::LeftBrace, "Expect '{' before method body.")?;
        let body = self.block()?;

        Ok(Decl::Function(name, params, return_type, body))
    }

    fn function_declaration(&mut self) -> PulseResult<Decl> {
        let name_token = self.consume_identifier("Expect function name.")?;
        let name = if let Token::Identifier(s) = name_token {
            s
        } else {
            unreachable!()
        };

        self.consume(Token::LeftParen, "Expect '(' after function name.")?;
        let mut params = Vec::new();
        if !self.check(Token::RightParen) {
            loop {
                let p_name_token = self.consume_identifier("Expect parameter name.")?;
                let p_name = if let Token::Identifier(s) = p_name_token {
                    s
                } else {
                    unreachable!()
                };

                let mut type_annotation = None;
                if self.matches(Token::Colon)? {
                    type_annotation = Some(self.parse_type()?);
                }

                params.push(TypedParam {
                    name: p_name,
                    type_annotation,
                });
                if !self.matches(Token::Comma)? {
                    break;
                }
            }
        }
        self.consume(Token::RightParen, "Expect ')' after parameters.")?;

        let mut return_type = None;
        if self.matches(Token::Arrow)? {
            return_type = Some(self.parse_type()?);
        }

        self.consume(Token::LeftBrace, "Expect '{' before function body.")?;
        let body = self.block()?;

        Ok(Decl::Function(name, params, return_type, body))
    }

    fn var_declaration(&mut self) -> PulseResult<Decl> {
        let name_token = self.consume_identifier("Expect variable name.")?;
        let name = if let Token::Identifier(s) = name_token {
            s
        } else {
            unreachable!()
        };

        let mut type_annotation = None;
        if self.matches(Token::Colon)? {
            type_annotation = Some(self.parse_type()?);
        }

        let mut initializer = None;
        if self.matches(Token::Equal)? {
            initializer = Some(self.expression()?);
        }
        self.consume(Token::Semicolon, "Expect ';' after variable declaration.")?;

        Ok(Decl::Stmt(Stmt::Let(name, type_annotation, initializer)))
    }

    fn const_declaration(&mut self) -> PulseResult<Decl> {
        let name_token = self.consume_identifier("Expect const name.")?;
        let name = if let Token::Identifier(s) = name_token {
            s
        } else {
            unreachable!()
        };

        let mut type_annotation = None;
        if self.matches(Token::Colon)? {
            type_annotation = Some(self.parse_type()?);
        }

        self.consume(Token::Equal, "Expect '=' after const declaration.")?;
        let initializer = self.expression()?;
        self.consume(Token::Semicolon, "Expect ';' after const declaration.")?;

        Ok(Decl::Stmt(Stmt::Const(name, type_annotation, initializer)))
    }

    fn actor_declaration(&mut self) -> PulseResult<Decl> {
        let name_token = self.consume_identifier("Expect actor name.")?;
        let name = if let Token::Identifier(s) = name_token {
            s
        } else {
            unreachable!()
        };

        self.consume(Token::LeftBrace, "Expect '{' before actor body.")?;
        let body = self.block()?;
        Ok(Decl::Actor(name, body))
    }

    fn shared_memory_declaration(&mut self) -> PulseResult<Decl> {
        self.consume(Token::Memory, "Expect 'memory' after 'shared'.")?;
        let name_token = self.consume_identifier("Expect shared memory name.")?;
        let name = if let Token::Identifier(s) = name_token {
            s
        } else {
            unreachable!()
        };
        self.consume(Token::Equal, "Expect '=' after shared memory name.")?;
        let init = self.expression()?;
        self.consume(
            Token::Semicolon,
            "Expect ';' after shared memory declaration.",
        )?;
        Ok(Decl::SharedMemory(name, init))
    }

    fn atomic_declaration(&mut self) -> PulseResult<Decl> {
        let name_token = self.consume_identifier("Expect atomic variable name.")?;
        let name = if let Token::Identifier(s) = name_token {
            s
        } else {
            unreachable!()
        };
        self.consume(Token::Equal, "Expect '=' after atomic variable name.")?;
        let init = self.expression()?;
        self.consume(Token::Semicolon, "Expect ';' after atomic declaration.")?;
        Ok(Decl::Stmt(Stmt::Let(name, None, Some(init))))
    }

    // --- Statements ---

    fn statement(&mut self) -> PulseResult<Stmt> {
        if self.matches(Token::If)? {
            self.if_statement()
        } else if self.matches(Token::For)? {
            self.for_statement()
        } else if self.matches(Token::While)? {
            self.while_statement()
        } else if self.matches(Token::Break)? {
            self.break_statement()
        } else if self.matches(Token::Continue)? {
            self.continue_statement()
        } else if self.matches(Token::Try)? {
            self.try_catch_statement()
        } else if self.matches(Token::Throw)? {
            self.throw_statement()
        } else if self.matches(Token::Lock)? {
            self.lock_statement()
        } else if self.matches(Token::Unlock)? {
            self.unlock_statement()
        } else if self.matches(Token::Fence)?
            || self.matches(Token::Acquire)?
            || self.matches(Token::Release)?
        {
            self.empty_unit_statement()
        } else if self.matches(Token::Spawn)? {
            self.spawn_statement()
        } else if self.matches(Token::Return)? {
            self.return_statement()
        } else if self.matches(Token::LeftBrace)? {
            Ok(Stmt::Block(self.block()?))
        } else if self.matches(Token::Send)? {
            self.send_statement()
        } else if self.matches(Token::Receive)? {
            self.receive_statement()
        } else if self.matches(Token::Link)? {
            self.link_statement()
        } else if self.matches(Token::Monitor)? {
            self.monitor_statement()
        } else if self.matches(Token::Match)? {
            self.match_statement()
        } else {
            self.expression_statement()
        }
    }

    fn match_statement(&mut self) -> PulseResult<Stmt> {
        let subject = self.expression()?;
        self.consume(Token::LeftBrace, "Expect '{' after match subject.")?;

        let mut arms = Vec::new();
        while !self.check(Token::RightBrace) && !self.is_at_end() {
            while self.matches(Token::Comma)? {} // skip leading commas
            if self.check(Token::RightBrace) { break; }
            let pattern = self.parse_match_pattern()?;
            self.consume(Token::FatArrow, "Expect '=>' after pattern.")?;

            let body = if self.check(Token::LeftBrace) {
                // Block body: { statements... }
                self.consume(Token::LeftBrace, "Expect '{'")?;
                let mut stmts = Vec::new();
                while !self.check(Token::RightBrace) && !self.is_at_end() {
                    stmts.push(self.statement()?);
                }
                self.consume(Token::RightBrace, "Expect '}'")?;
                Stmt::Block(stmts)
            } else if self.check(Token::If)
                || self.check(Token::While)
                || self.check(Token::Return)
                || self.check(Token::Let)
                || self.check(Token::Const)
            {
                // Other statement bodies
                self.statement()?
            } else {
                // Expression body: expr
                let expr = self.expression()?;
                Stmt::Expression(expr)
            };

            arms.push((pattern, body));

            while self.matches(Token::Comma)? {} // consume all optional commas
            if self.check(Token::RightBrace) {
                break;
            }
        }

        self.consume(Token::RightBrace, "Expect '}' after match arms.")?;
        if self.matches(Token::Semicolon)? {} // match usually doesn't need semicolon
        Ok(Stmt::Match(subject, arms))
    }

    fn parse_match_pattern(&mut self) -> PulseResult<MatchPattern> {
        if self.matches(Token::Underscore)? {
            Ok(MatchPattern::Wildcard)
        } else if self.check_identifier() {
            let token = self.advance_and_return()?;
            if let Token::Identifier(s) = token {
                Ok(MatchPattern::Variable(s))
            } else {
                Ok(MatchPattern::Wildcard)
            }
        } else if self.is_literal_token() {
            let start = self.parse_literal_pattern()?;
            if self.matches(Token::DotDot)? {
                let end = self.parse_literal_pattern()?;
                Ok(MatchPattern::Range(start, end))
            } else {
                Ok(MatchPattern::Literal(start))
            }
        } else {
            Err(self.error("Expect pattern (wildcard, variable, or literal)."))
        }
    }

    fn check_identifier(&self) -> bool {
        matches!(self.current, Token::Identifier(_))
    }

    fn is_literal_token(&self) -> bool {
        matches!(
            self.current,
            Token::Int(_)
                | Token::Float(_)
                | Token::String(_)
                | Token::True
                | Token::False
                | Token::Nil
        )
    }

    fn parse_literal_pattern(&mut self) -> PulseResult<Constant> {
        let token = self.advance_and_return()?;
        match token {
            Token::Int(n) => Ok(Constant::Int(n)),
            Token::Float(f) => Ok(Constant::Float(f)),
            Token::String(s) => Ok(Constant::String(s)),
            Token::True => Ok(Constant::Bool(true)),
            Token::False => Ok(Constant::Bool(false)),
            Token::Nil => Ok(Constant::Unit),
            _ => Err(self.error("Expect literal pattern.")),
        }
    }

    fn try_catch_statement(&mut self) -> PulseResult<Stmt> {
        self.consume(Token::LeftBrace, "Expect '{' after 'try'.")?;
        let try_block = Box::new(Stmt::Block(self.block()?));

        self.consume(Token::Catch, "Expect 'catch' after try block.")?;
        let error_var_token = self.consume_identifier("Expect error variable name.")?;
        let error_var = if let Token::Identifier(s) = error_var_token {
            s
        } else {
            unreachable!()
        };

        self.consume(Token::LeftBrace, "Expect '{' before catch block.")?;
        let catch_block = Box::new(Stmt::Block(self.block()?));

        Ok(Stmt::Try(try_block, error_var, catch_block))
    }

    fn throw_statement(&mut self) -> PulseResult<Stmt> {
        let expr = self.expression()?;
        self.consume(Token::Semicolon, "Expect ';' after throw.")?;
        Ok(Stmt::Throw(expr))
    }

    fn spawn_statement(&mut self) -> PulseResult<Stmt> {
        let expr = if self.check(Token::LeftBrace) {
            self.advance()?;
            let body = self.block()?;
            Expr::Closure("__spawn_body".to_string(), Vec::new(), None, body)
        } else {
            self.expression()?
        };
        self.consume(Token::Semicolon, "Expect ';' after spawn.")?;
        Ok(Stmt::Spawn(expr))
    }

    fn send_statement(&mut self) -> PulseResult<Stmt> {
        // send target, message OR send(target, message)
        let has_paren = self.matches(Token::LeftParen)?;
        let target = self.expression()?;
        self.consume(Token::Comma, "Expect ',' after target actor.")?;
        let message = self.expression()?;
        if has_paren {
            self.consume(Token::RightParen, "Expect ')' after message.")?;
        }
        self.consume(Token::Semicolon, "Expect ';' after send.")?;
        Ok(Stmt::Send(target, message))
    }

    fn receive_expr(&mut self) -> PulseResult<Expr> {
        // receive { pattern => expr, ... } or just receive or receive()
        if self.matches(Token::LeftParen)? {
            self.consume(Token::RightParen, "Expect ')' after 'receive'.")?;
            return Ok(Expr::Receive(vec![]));
        } else if !self.check(Token::LeftBrace) {
            return Ok(Expr::Receive(vec![]));
        }

        self.consume(Token::LeftBrace, "Expect '{' after 'receive'.")?;
        let mut arms = Vec::new();
        while !self.check(Token::RightBrace) && !self.is_at_end() {
            // Parse pattern (can be identifier, literal, or _)
            let pattern = if self.matches(Token::Underscore)? {
                Pattern::Wildcard
            } else if self.check(Token::Identifier("".to_string())) {
                let token = self.advance_and_return()?;
                if let Token::Identifier(s) = token {
                    Pattern::Variable(s)
                } else {
                    Pattern::Wildcard
                }
            } else {
                // Literal pattern
                let expr = self.expression()?;
                Pattern::Literal(expr)
            };

            self.consume(Token::FatArrow, "Expect '=>' after pattern.")?;
            let body = self.expression()?;
            arms.push((pattern, body));

            if self.check(Token::Comma) {
                self.advance()?;
            }
        }

        self.consume(Token::RightBrace, "Expect '}' after receive arms.")?;
        Ok(Expr::Receive(arms))
    }

    fn receive_statement(&mut self) -> PulseResult<Stmt> {
        // receive { pattern => expr, ... }
        self.consume(Token::LeftBrace, "Expect '{' after 'receive'.")?;

        let mut arms = Vec::new();
        while !self.check(Token::RightBrace) && !self.is_at_end() {
            // Parse pattern (can be identifier, literal, or _)
            let pattern = if self.matches(Token::Underscore)? {
                Pattern::Wildcard
            } else if self.check(Token::Identifier("".to_string())) {
                let token = self.advance_and_return()?;
                if let Token::Identifier(s) = token {
                    Pattern::Variable(s)
                } else {
                    Pattern::Wildcard
                }
            } else {
                // Literal pattern
                let expr = self.expression()?;
                Pattern::Literal(expr)
            };

            self.consume(Token::FatArrow, "Expect '=>' after pattern.")?;
            let body = self.expression()?;

            arms.push((pattern, body));

            if !self.matches(Token::Comma)? && !self.check(Token::RightBrace) {
                return Err(self.error("Expect ',' or '}' after receive arm."));
            }
        }

        self.consume(Token::RightBrace, "Expect '}' after receive block.")?;
        if self.matches(Token::Semicolon)? {}

        // Convert to a special receive expression statement
        Ok(Stmt::Expression(Expr::Receive(arms)))
    }

    fn link_statement(&mut self) -> PulseResult<Stmt> {
        self.consume(Token::LeftParen, "Expect '(' after 'link'.")?;
        let target = self.expression()?;
        self.consume(Token::RightParen, "Expect ')' after target.")?;
        self.consume(Token::Semicolon, "Expect ';' after link.")?;
        Ok(Stmt::Link(target))
    }

    fn monitor_statement(&mut self) -> PulseResult<Stmt> {
        self.consume(Token::LeftParen, "Expect '(' after 'monitor'.")?;
        let target = self.expression()?;
        self.consume(Token::RightParen, "Expect ')' after target.")?;
        self.consume(Token::Semicolon, "Expect ';' after monitor.")?;
        Ok(Stmt::Monitor(target))
    }

    fn block(&mut self) -> PulseResult<Vec<Stmt>> {
        let mut stmts = Vec::new();
        while !self.check(Token::RightBrace) && !self.is_at_end() {
            match self.declaration()? {
                Decl::Stmt(s) => stmts.push(s),
                Decl::Function(name, params, ret, body) => {
                    let closure = Expr::Closure(name.clone(), params, ret, body);
                    stmts.push(Stmt::Let(name, None, Some(closure)));
                }
                Decl::Class(name, parent, methods) => {
                    // Class declaration in block - convert to statement
                    stmts.push(Stmt::Expression(Expr::ClassLiteral(name, parent, methods)));
                }
                Decl::Actor(name, body) => {
                    let _ = (name, body);
                    stmts.push(Stmt::Expression(Expr::Literal(Constant::Unit)));
                }
                Decl::SharedMemory(_name, _expr) => {
                    stmts.push(Stmt::Expression(Expr::Literal(Constant::Unit)));
                }
            }
        }
        self.consume(Token::RightBrace, "Expect '}' after block.")?;
        Ok(stmts)
    }

    fn if_statement(&mut self) -> PulseResult<Stmt> {
        let condition = self.expression()?;

        // Check if condition is a type guard for type narrowing
        let narrowing = self.extract_type_narrowing(&condition);

        let then_branch = Box::new(self.statement()?);
        let mut else_branch = None;
        if self.matches(Token::Else)? {
            if self.matches(Token::If)? {
                else_branch = Some(Box::new(self.if_statement()?));
            } else {
                else_branch = Some(Box::new(self.statement()?));
            }
        }
        Ok(Stmt::If(
            Box::new(condition),
            then_branch,
            else_branch,
            narrowing,
        ))
    }

    /// Extract type narrowing information from a type guard condition
    fn extract_type_narrowing(&self, condition: &Expr) -> Option<crate::ast::TypeNarrowing> {
        match condition {
            Expr::TypeGuard(var_expr, ty) => {
                if let Expr::Variable(name) = var_expr.as_ref() {
                    Some(crate::ast::TypeNarrowing {
                        var_name: name.clone(),
                        narrowed_type: ty.clone(),
                        else_type: None, // Would need union type info
                    })
                } else {
                    None
                }
            }
            Expr::Binary(left, BinOp::And, right) => {
                // Check left side for type guard
                if let Some(n) = self.extract_type_narrowing(left) {
                    Some(n)
                } else {
                    self.extract_type_narrowing(right)
                }
            }
            _ => None,
        }
    }

    fn while_statement(&mut self) -> PulseResult<Stmt> {
        let condition = self.expression()?;
        let body = Box::new(self.statement()?);
        Ok(Stmt::While(condition, body))
    }

    fn for_statement(&mut self) -> PulseResult<Stmt> {
        // C-style: for (init; cond; update) stmt
        if self.matches(Token::LeftParen)? {
            let init = if self.matches(Token::Semicolon)? {
                None
            } else if self.matches(Token::Let)? {
                Some(Box::new(self.for_let_initializer()?))
            } else if self.matches(Token::Const)? {
                Some(Box::new(self.for_const_initializer()?))
            } else {
                let expr = self.expression()?;
                self.consume(Token::Semicolon, "Expect ';' after for initializer.")?;
                Some(Box::new(Stmt::Expression(expr)))
            };

            let cond = if !self.check(Token::Semicolon) {
                Some(self.expression()?)
            } else {
                None
            };
            self.consume(Token::Semicolon, "Expect ';' after loop condition.")?;

            let update = if !self.check(Token::RightParen) {
                Some(self.expression()?)
            } else {
                None
            };
            self.consume(Token::RightParen, "Expect ')' after for clauses.")?;

            let body = Box::new(self.statement()?);
            return Ok(Stmt::For(init, cond, update, body));
        }

        // For-in style: for item in iterable stmt
        let var_token = self.consume_identifier("Expect loop variable after 'for'.")?;
        let var_name = if let Token::Identifier(s) = var_token {
            s
        } else {
            unreachable!()
        };
        self.consume(Token::In, "Expect 'in' after loop variable.")?;
        let iterable = self.expression()?;
        let body = Box::new(self.statement()?);

        // Desugar to a block so type-checking can still see iterable expression.
        Ok(Stmt::Block(vec![
            Stmt::Expression(iterable),
            Stmt::For(
                Some(Box::new(Stmt::Let(var_name, None, None))),
                None,
                None,
                body,
            ),
        ]))
    }

    fn for_let_initializer(&mut self) -> PulseResult<Stmt> {
        let name_token = self.consume_identifier("Expect variable name.")?;
        let name = if let Token::Identifier(s) = name_token {
            s
        } else {
            unreachable!()
        };

        let mut type_annotation = None;
        if self.matches(Token::Colon)? {
            type_annotation = Some(self.parse_type()?);
        }

        let mut initializer = None;
        if self.matches(Token::Equal)? {
            initializer = Some(self.expression()?);
        }
        self.consume(Token::Semicolon, "Expect ';' after variable declaration.")?;
        Ok(Stmt::Let(name, type_annotation, initializer))
    }

    fn for_const_initializer(&mut self) -> PulseResult<Stmt> {
        let name_token = self.consume_identifier("Expect const name.")?;
        let name = if let Token::Identifier(s) = name_token {
            s
        } else {
            unreachable!()
        };
        let mut type_annotation = None;
        if self.matches(Token::Colon)? {
            type_annotation = Some(self.parse_type()?);
        }
        self.consume(Token::Equal, "Expect '=' after const declaration.")?;
        let initializer = self.expression()?;
        self.consume(Token::Semicolon, "Expect ';' after const declaration.")?;
        Ok(Stmt::Const(name, type_annotation, initializer))
    }

    fn break_statement(&mut self) -> PulseResult<Stmt> {
        self.consume(Token::Semicolon, "Expect ';' after 'break'.")?;
        Ok(Stmt::Break)
    }

    fn continue_statement(&mut self) -> PulseResult<Stmt> {
        self.consume(Token::Semicolon, "Expect ';' after 'continue'.")?;
        Ok(Stmt::Continue)
    }

    fn lock_statement(&mut self) -> PulseResult<Stmt> {
        let _target = self.expression()?;
        self.consume(Token::Semicolon, "Expect ';' after lock target.")?;
        Ok(Stmt::Expression(Expr::Literal(Constant::Unit)))
    }

    fn unlock_statement(&mut self) -> PulseResult<Stmt> {
        let _target = self.expression()?;
        self.consume(Token::Semicolon, "Expect ';' after unlock target.")?;
        Ok(Stmt::Expression(Expr::Literal(Constant::Unit)))
    }

    fn empty_unit_statement(&mut self) -> PulseResult<Stmt> {
        self.consume(Token::Semicolon, "Expect ';' after memory-order keyword.")?;
        Ok(Stmt::Expression(Expr::Literal(Constant::Unit)))
    }

    fn return_statement(&mut self) -> PulseResult<Stmt> {
        let mut value = None;
        if !self.check(Token::Semicolon) {
            value = Some(self.expression()?);
        }
        self.consume(Token::Semicolon, "Expect ';' after return value.")?;
        Ok(Stmt::Return(value))
    }

    fn expression_statement(&mut self) -> PulseResult<Stmt> {
        let expr = self.expression()?;
        self.consume(Token::Semicolon, "Expect ';' after expression.")?;
        Ok(Stmt::Expression(expr))
    }

    // --- Expressions (Pratt Parsing) ---

    fn expression(&mut self) -> PulseResult<Expr> {
        self.parse_precedence(PrecedenceV2::Assignment)
    }

    fn parse_precedence(&mut self, precedence: PrecedenceV2) -> PulseResult<Expr> {
        self.advance()?;
        let mut left = self.prefix_rule(self.previous.clone())?;

        while precedence <= self.get_precedence(&self.current) {
            self.advance()?;
            left = self.infix_rule(left, self.previous.clone())?;
        }

        Ok(left)
    }

    fn prefix_rule(&mut self, token: Token) -> PulseResult<Expr> {
        match token {
            Token::True => Ok(Expr::Literal(Constant::Bool(true))),
            Token::False => Ok(Expr::Literal(Constant::Bool(false))),
            Token::Nil => Ok(Expr::Literal(Constant::Unit)),
            Token::Int(i) => Ok(Expr::Literal(Constant::Int(i))),
            Token::Float(f) => Ok(Expr::Literal(Constant::Float(f))),
            Token::String(s) => Ok(Expr::Literal(Constant::String(s))),
            Token::Identifier(s) => Ok(Expr::Variable(s)),
            Token::Bang => {
                let expr = self.parse_precedence(PrecedenceV2::Unary)?;
                Ok(Expr::Unary(UnOp::Not, Box::new(expr)))
            }
            Token::Minus => {
                let expr = self.parse_precedence(PrecedenceV2::Unary)?;
                Ok(Expr::Unary(UnOp::Neg, Box::new(expr)))
            }
            Token::Tilde => {
                let expr = self.parse_precedence(PrecedenceV2::Unary)?;
                Ok(Expr::Unary(UnOp::BitNot, Box::new(expr)))
            }
            Token::LeftParen => {
                let expr = self.expression()?;
                self.consume(Token::RightParen, "Expect ')' after expression.")?;
                Ok(expr)
            }
            Token::LeftBracket => self.list_literal(),
            Token::LeftBrace => self.map_or_block_literal(),
            Token::Fn => self.lambda_literal(),
            Token::This => Ok(Expr::This),
            Token::Receive => self.receive_expr(),
            Token::Print => {
                let mut exprs = Vec::new();
                let has_parens = self.matches(Token::LeftParen)?;
                if has_parens && self.check(Token::RightParen) {
                    // empty
                } else {
                    exprs.push(self.expression()?);
                    while self.matches(Token::Comma)? {
                        if has_parens && self.check(Token::RightParen) {
                            break;
                        }
                        exprs.push(self.expression()?);
                    }
                }
                if has_parens {
                    self.consume(Token::RightParen, "Expect ')' after print arguments.")?;
                }

                // Reconstruct print as a method/function call ast node or specific PrintExpr,
                // but wait, does AST have a Print expression node?
                // If not, we might need to map it to a Call(Variable("print"), args)
                // Let's assume there is no Print expression node and we must use Stmt::Print or desugar it to a Call.
                // Actually, wait, let's map it to Expr::Call(Box::new(Expr::Variable("print".to_string())), exprs)
                Ok(Expr::Call(Box::new(Expr::Variable("print".to_string())), exprs))
            }
            Token::Println => {
                let mut exprs = Vec::new();
                let has_parens = self.matches(Token::LeftParen)?;
                if has_parens && self.check(Token::RightParen) {
                    // empty
                } else {
                    exprs.push(self.expression()?);
                    while self.matches(Token::Comma)? {
                        if has_parens && self.check(Token::RightParen) {
                            break;
                        }
                        exprs.push(self.expression()?);
                    }
                }
                if has_parens {
                    self.consume(Token::RightParen, "Expect ')' after println arguments.")?;
                }
                Ok(Expr::Call(Box::new(Expr::Variable("println".to_string())), exprs))
            }
            Token::Super => {
                self.consume(Token::Dot, "Expect '.' after 'super'.")?;
                let method_token = self.consume_identifier("Expect superclass method name.")?;
                let method = if let Token::Identifier(s) = method_token {
                    s
                } else {
                    unreachable!()
                };
                Ok(Expr::Super(method))
            }
            Token::Import => self.import_expression_placeholder(),
            Token::Spawn => self.spawn_expression(),
            Token::Match => self.match_expression_placeholder(),
            _ => Err(self.error(format!("Expect expression, got {:?}.", token))),
        }
    }

    fn import_expression_placeholder(&mut self) -> PulseResult<Expr> {
        // ParserV2 pre-pass support for `let x = import "path";`.
        // Runtime import semantics are lowered in the bytecode compiler pass.
        let _path_token = self.consume(
            Token::String("".to_string()),
            "Expect module path string after 'import'.",
        )?;
        // Model imported module values as map-like objects in the AST pre-pass.
        Ok(Expr::Map(Vec::new()))
    }

    fn match_expression_placeholder(&mut self) -> PulseResult<Expr> {
        let _subject = self.expression()?;
        self.consume(Token::LeftBrace, "Expect '{' after match subject.")?;

        while !self.check(Token::RightBrace) && !self.is_at_end() {
            let _ = self.parse_match_pattern()?;
            self.consume(Token::FatArrow, "Expect '=>' after pattern.")?;

            if self.check(Token::LeftBrace) {
                self.advance()?;
                let _ = self.block()?;
            } else if self.check(Token::Print)
                || self.check(Token::If)
                || self.check(Token::While)
                || self.check(Token::Return)
                || self.check(Token::Let)
                || self.check(Token::Const)
            {
                let _ = self.statement()?;
            } else {
                let _ = self.expression()?;
            }

            if !self.matches(Token::Comma)? && !self.check(Token::RightBrace) {
                return Err(self.error("Expect ',' or '}' after match arm."));
            }
        }

        self.consume(Token::RightBrace, "Expect '}' after match arms.")?;
        Ok(Expr::Literal(Constant::Unit))
    }

    fn list_literal(&mut self) -> PulseResult<Expr> {
        let mut elements = Vec::new();

        if !self.check(Token::RightBracket) {
            loop {
                elements.push(self.expression()?);
                if !self.matches(Token::Comma)? {
                    break;
                }
            }
        }

        self.consume(Token::RightBracket, "Expect ']' after list elements.")?;
        Ok(Expr::List(elements))
    }

    fn map_or_block_literal(&mut self) -> PulseResult<Expr> {
        // Lookahead to determine if this is a map or a block
        // Maps have the form { "key": value } or { ident: value }
        // Blocks have statements

        // Simple heuristic: if next token is a string or identifier followed by colon (not ::), it's a map
        let is_map = self.check_map_pattern();

        if is_map {
            self.map_literal()
        } else {
            // This is actually a block, but we're in prefix position
            // Return an error suggesting the correct syntax
            Err(self.error(
                "Unexpected '{' in expression. Use 'fn() { ... }' for closures or use a let binding.",
            ))
        }
    }

    fn check_map_pattern(&self) -> bool {
        // Simple lookahead - check if we have { "key" or { identifier:
        matches!(self.current, Token::String(_) | Token::Identifier(_))
    }

    fn map_literal(&mut self) -> PulseResult<Expr> {
        let mut entries = Vec::new();

        if !self.check(Token::RightBrace) {
            loop {
                // Parse key (string or identifier)
                let key = if let Token::String(s) = &self.current {
                    let s = s.clone();
                    self.advance().ok();
                    Expr::Literal(Constant::String(s))
                } else if let Token::Identifier(_) = &self.current {
                    let token = self.advance_and_return().ok().unwrap_or(Token::Nil);
                    if let Token::Identifier(s) = token {
                        Expr::Literal(Constant::String(s))
                    } else {
                        return Err(self.error("Expect identifier or string as map key."));
                    }
                } else {
                    return Err(self.error("Expect string or identifier as map key."));
                };

                self.consume(Token::Colon, "Expect ':' after map key.")?;
                let value = self.expression()?;
                entries.push((key, value));

                if !self.matches(Token::Comma)? {
                    break;
                }
            }
        }

        self.consume(Token::RightBrace, "Expect '}' after map entries.")?;
        Ok(Expr::Map(entries))
    }

    fn lambda_literal(&mut self) -> PulseResult<Expr> {
        self.consume(Token::LeftParen, "Expect '(' after 'fn'.")?;
        let mut params = Vec::new();

        if !self.check(Token::RightParen) {
            loop {
                let p_name_token = self.consume_identifier("Expect parameter name.")?;
                let p_name = if let Token::Identifier(s) = p_name_token {
                    s
                } else {
                    unreachable!()
                };

                let mut type_annotation = None;
                if self.matches(Token::Colon)? {
                    type_annotation = Some(self.parse_type()?);
                }

                params.push(TypedParam {
                    name: p_name,
                    type_annotation,
                });
                if !self.matches(Token::Comma)? {
                    break;
                }
            }
        }
        self.consume(Token::RightParen, "Expect ')' after parameters.")?;

        let mut return_type = None;
        if self.matches(Token::Arrow)? {
            return_type = Some(self.parse_type()?);
        }

        self.consume(Token::LeftBrace, "Expect '{' before lambda body.")?;
        let body = self.block()?;

        // Generate a unique name for the closure
        let name = format!("__lambda_{}_{}", self.current_line(), self.previous_line);
        Ok(Expr::Closure(name, params, return_type, body))
    }

    fn spawn_expression(&mut self) -> PulseResult<Expr> {
        // spawn ActorName(args) or spawn fn() { ... }
        if self.matches(Token::Fn)? {
            let expr = self.lambda_literal()?;
            return Ok(Expr::Spawn(Box::new(expr)));
        }

        let expr = if self.check(Token::LeftBrace) {
            self.advance()?;
            let body = self.block()?;
            Expr::Closure("__spawn_body".to_string(), Vec::new(), None, body)
        } else {
            self.expression()?
        };
        Ok(Expr::Spawn(Box::new(expr)))
    }

    fn infix_rule(&mut self, left: Expr, token: Token) -> PulseResult<Expr> {
        match token {
            Token::Plus
            | Token::Minus
            | Token::Star
            | Token::Slash
            | Token::Percent
            | Token::StarStar
            | Token::Ampersand
            | Token::Pipe
            | Token::Caret
            | Token::ShiftLeft
            | Token::ShiftRight
            | Token::EqualEqual
            | Token::BangEqual
            | Token::LeftAngle
            | Token::RightAngle
            | Token::Less
            | Token::LessEqual
            | Token::Greater
            | Token::GreaterEqual
            | Token::LogicalAnd
            | Token::LogicalOr
            | Token::And
            | Token::Or => {
                let op = self.token_to_binop(&token);
                let precedence = self.get_precedence(&token).next();
                let right = self.parse_precedence(precedence)?;
                Ok(Expr::Binary(Box::new(left), op, Box::new(right)))
            }
            Token::LeftParen => {
                let mut args = Vec::new();
                if !self.check(Token::RightParen) {
                    loop {
                        args.push(self.expression()?);
                        if !self.matches(Token::Comma)? {
                            break;
                        }
                    }
                }
                self.consume(Token::RightParen, "Expect ')' after arguments.")?;
                Ok(Expr::Call(Box::new(left), args))
            }
            Token::Dot => {
                let name_token = self.consume_identifier("Expect property name after '.'.")?;
                let name = if let Token::Identifier(s) = name_token {
                    s
                } else {
                    unreachable!()
                };

                // Check if it's a method call (followed by ()
                if self.check(Token::LeftParen) {
                    // Method call: obj.method(args)
                    self.advance()?;
                    let mut args = Vec::new();

                    if !self.check(Token::RightParen) {
                        loop {
                            args.push(self.expression()?);
                            if !self.matches(Token::Comma)? {
                                break;
                            }
                        }
                    }
                    self.consume(Token::RightParen, "Expect ')' after arguments.")?;
                    let method_name = name.clone();
                    Ok(Expr::MethodCall(Box::new(left), method_name, args))
                } else {
                    // Property access: obj.property
                    Ok(Expr::Get(Box::new(left), name))
                }
            }
            Token::LeftBracket => {
                // Index expression: obj[index]
                let index = self.expression()?;
                self.consume(Token::RightBracket, "Expect ']' after index.")?;
                Ok(Expr::Index(Box::new(left), Box::new(index)))
            }
            Token::Bang if self.previous == Token::Bang => {
                // Send operator: actor ! message
                let right = self.parse_precedence(PrecedenceV2::Unary)?;
                Ok(Expr::Send(Box::new(left), Box::new(right)))
            }
            Token::Equal => {
                // Assignment
                let value = self.parse_precedence(PrecedenceV2::Assignment)?;
                match left {
                    Expr::Variable(name) => Ok(Expr::Assign(name, Box::new(value))),
                    Expr::Get(obj, name) => Ok(Expr::Set(obj, name, Box::new(value))),
                    Expr::Index(obj, index) => Ok(Expr::IndexSet(obj, index, Box::new(value))),
                    _ => Err(self.error("Invalid assignment target.")),
                }
            }
            Token::Is => {
                // Type guard: x is Type
                let ty = self.parse_type()?;
                Ok(Expr::TypeGuard(Box::new(left), ty))
            }
            _ => Err(self.error(format!("Unexpected infix token {:?}.", token))),
        }
    }

    fn token_to_binop(&self, token: &Token) -> BinOp {
        match token {
            Token::Plus => BinOp::Add,
            Token::Minus => BinOp::Sub,
            Token::Star => BinOp::Mul,
            Token::Slash => BinOp::Div,
            Token::Percent => BinOp::Mod,
            Token::StarStar => BinOp::Pow,
            Token::EqualEqual => BinOp::Eq,
            Token::BangEqual => BinOp::Ne,
            Token::LeftAngle => BinOp::Lt,
            Token::RightAngle => BinOp::Gt,
            Token::Less => BinOp::Lt,
            Token::LessEqual => BinOp::Le,
            Token::Greater => BinOp::Gt,
            Token::GreaterEqual => BinOp::Ge,
            Token::Ampersand => BinOp::BitAnd,
            Token::Pipe => BinOp::BitOr,
            Token::Caret => BinOp::BitXor,
            Token::ShiftLeft => BinOp::Shl,
            Token::ShiftRight => BinOp::Shr,
            Token::LogicalAnd => BinOp::And,
            Token::LogicalOr => BinOp::Or,
            Token::And => BinOp::And,
            Token::Or => BinOp::Or,
            _ => unreachable!(),
        }
    }

    fn get_precedence(&self, token: &Token) -> PrecedenceV2 {
        match token {
            Token::Or => PrecedenceV2::Or,
            Token::And => PrecedenceV2::And,
            Token::EqualEqual | Token::BangEqual => PrecedenceV2::Equality,
            Token::LeftAngle
            | Token::Less
            | Token::LessEqual
            | Token::RightAngle
            | Token::Greater
            | Token::GreaterEqual => PrecedenceV2::Comparison,
            Token::Is => PrecedenceV2::Comparison, // Type guards have comparison precedence
            Token::Plus
            | Token::Minus
            | Token::Ampersand
            | Token::Pipe
            | Token::Caret
            | Token::ShiftLeft
            | Token::ShiftRight => PrecedenceV2::Term,
            Token::Star | Token::Slash | Token::Percent | Token::StarStar => PrecedenceV2::Factor,
            Token::LeftParen | Token::LeftBracket => PrecedenceV2::Call,
            Token::Dot => PrecedenceV2::Call,
            Token::Bang => PrecedenceV2::Unary, // For send operator
            Token::Equal => PrecedenceV2::Assignment,
            Token::LogicalAnd => PrecedenceV2::And,
            Token::LogicalOr => PrecedenceV2::Or,
            _ => PrecedenceV2::None,
        }
    }

    // --- Helpers ---

    fn consume_identifier(&mut self, message: &str) -> PulseResult<Token> {
        match &self.current {
            Token::Identifier(_) => {
                let t = self.current.clone();
                self.advance()?;
                Ok(t)
            }
            _ => Err(self.error(message)),
        }
    }

    fn parse_type(&mut self) -> PulseResult<Type> {
        let mut ty = self.parse_base_type()?;

        // Handle union types: Int | String
        while self.matches(Token::Pipe)? {
            let right = self.parse_base_type()?;
            ty = match ty {
                Type::Union(mut types) => {
                    match right {
                        Type::Union(other_types) => {
                            types.extend(other_types);
                        }
                        _ => types.push(right),
                    }
                    Type::Union(types)
                }
                _ => Type::Union(vec![ty, right]),
            };
        }

        Ok(ty)
    }

    fn parse_base_type(&mut self) -> PulseResult<Type> {
        let token = self.advance_and_return()?;
        let mut ty = match token {
            Token::Identifier(s) => {
                match s.as_str() {
                    "Int" => Ok(Type::Int),
                    "Float" => Ok(Type::Float),
                    "Bool" => Ok(Type::Bool),
                    "String" => Ok(Type::String),
                    "Unit" => Ok(Type::Unit),
                    "Any" => Ok(Type::Any),
                    "Pid" => Ok(Type::Pid),
                    "Atomic" => Ok(Type::Atomic),
                    "Option" => {
                        // Parse Option<T>
                        self.consume(Token::LeftAngle, "Expect '<' after 'Option'.")?;
                        let inner = self.parse_type()?;
                        self.consume(Token::RightAngle, "Expect '>' after Option type parameter.")?;
                        Ok(Type::Option(Box::new(inner)))
                    }
                    _ => Ok(Type::Custom(s)),
                }
            }
            Token::TypeInt => Ok(Type::Int),
            Token::TypeFloat => Ok(Type::Float),
            Token::TypeBool => Ok(Type::Bool),
            Token::TypeString => Ok(Type::String),
            Token::TypeUnit => Ok(Type::Unit),
            Token::TypePid => Ok(Type::Pid),
            Token::TypeAny => Ok(Type::Any),
            Token::TypeAtomic => Ok(Type::Atomic),
            Token::TypeList => {
                // Parse List<T>
                self.consume(Token::LeftAngle, "Expect '<' after 'List'.")?;
                let inner = self.parse_type()?;
                self.consume(Token::RightAngle, "Expect '>' after List type parameter.")?;
                Ok(Type::List(Box::new(inner)))
            }
            Token::TypeMap => {
                // Parse Map<K, V>
                self.consume(Token::LeftAngle, "Expect '<' after 'Map'.")?;
                let key = self.parse_type()?;
                self.consume(Token::Comma, "Expect ',' between Map key and value types.")?;
                let value = self.parse_type()?;
                self.consume(Token::RightAngle, "Expect '>' after Map type parameters.")?;
                Ok(Type::Map(Box::new(key), Box::new(value)))
            }
            Token::TypeFn => {
                // Parse Fn<(T1, T2) -> R>
                self.consume(Token::LeftAngle, "Expect '<' after 'Fn'.")?;
                self.consume(Token::LeftParen, "Expect '(' for function parameters.")?;
                let mut params = Vec::new();
                if !self.check(Token::RightParen) {
                    loop {
                        params.push(self.parse_type()?);
                        if !self.matches(Token::Comma)? {
                            break;
                        }
                    }
                }
                self.consume(Token::RightParen, "Expect ')' after function parameters.")?;
                self.consume(Token::Arrow, "Expect '->' before return type.")?;
                let ret = self.parse_type()?;
                self.consume(Token::RightAngle, "Expect '>' after function type.")?;
                Ok(Type::Fn(params, Box::new(ret)))
            }
            _ => Err(self.error(format!("Expect type name, got {:?}.", token))),
        }?;

        // Handle generic type parameters for custom types: Custom<T>
        if self.matches(Token::LeftAngle)? {
            let generic_param = self.parse_type()?;
            self.consume(
                Token::RightAngle,
                "Expect '>' after generic type parameter.",
            )?;
            // Store as Custom with generic info
            if let Type::Custom(name) = &ty {
                ty = Type::Generic(format!("{}<{}>", name, generic_param));
            }
        }

        Ok(ty)
    }

    fn advance_and_return(&mut self) -> PulseResult<Token> {
        let t = self.current.clone();
        self.advance()?;
        Ok(t)
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
enum PrecedenceV2 {
    None,
    Assignment,
    Or,
    And,
    Equality,
    Comparison,
    Term,
    Factor,
    Unary,
    Call,
    Primary,
}

impl PrecedenceV2 {
    fn next(&self) -> Self {
        match self {
            PrecedenceV2::None => PrecedenceV2::Assignment,
            PrecedenceV2::Assignment => PrecedenceV2::Or,
            PrecedenceV2::Or => PrecedenceV2::And,
            PrecedenceV2::And => PrecedenceV2::Equality,
            PrecedenceV2::Equality => PrecedenceV2::Comparison,
            PrecedenceV2::Comparison => PrecedenceV2::Term,
            PrecedenceV2::Term => PrecedenceV2::Factor,
            PrecedenceV2::Factor => PrecedenceV2::Unary,
            PrecedenceV2::Unary => PrecedenceV2::Call,
            PrecedenceV2::Call => PrecedenceV2::Primary,
            PrecedenceV2::Primary => PrecedenceV2::Primary,
        }
    }
}

// Re-export Pattern from ast for use in parser
pub use crate::ast::Pattern;

// Additional Expr variants needed:
// - Expr::Assign(String, Box<Expr>) - for assignment
// - Expr::IndexSet(Box<Expr>, Box<Expr>, Box<Expr>) - for index assignment
// - Expr::ClassLiteral(String, Option<String>, Vec<Decl>) - for class expressions
// - Expr::MethodCall(Box<Expr>, String, Vec<Expr>) - for method calls
// - Expr::Receive(Vec<(Pattern, Expr)>) - for receive expressions
// - Expr::Spawn(Box<Expr>) - for spawn expression
// - Expr::Send(Box<Expr>, Box<Expr>) - for send expression

// Additional Stmt variants needed:
// - Stmt::Import(String, Option<String>) - for import with optional alias
