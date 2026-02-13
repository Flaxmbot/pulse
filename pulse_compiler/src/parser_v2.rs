use crate::lexer::{Lexer, Token};
use crate::ast::*;
use crate::types::{Type, TypedParam};
use pulse_core::{PulseError, PulseResult, Constant};

pub struct ParserV2<'a> {
    lexer: Lexer<'a>,
    current: Token,
    previous: Token,
}

impl<'a> ParserV2<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            lexer: Lexer::new(source),
            current: Token::Eof,
            previous: Token::Eof,
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

    fn advance(&mut self) -> PulseResult<()> {
        self.previous = self.current.clone();
        self.current = self.lexer.next_token()?;
        Ok(())
    }

    fn check(&self, token_type: Token) -> bool {
        self.current == token_type
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

    fn consume(&mut self, expected: Token, message: &str) -> PulseResult<Token> {
        if self.check(expected) {
            let t = self.current.clone();
            self.advance()?;
            Ok(t)
        } else {
            Err(PulseError::CompileError(message.to_string(), self.lexer.line))
        }
    }

    // --- Declarations ---

    fn declaration(&mut self) -> PulseResult<Decl> {
        if self.matches(Token::Let)? {
            self.var_declaration()
        } else if self.matches(Token::Fn)? || self.matches(Token::Def)? {
            self.function_declaration()
        } else {
            Ok(Decl::Stmt(self.statement()?))
        }
    }

    fn function_declaration(&mut self) -> PulseResult<Decl> {
        let name_token = self.consume_identifier("Expect function name.")?;
        let name = if let Token::Identifier(s) = name_token { s } else { unreachable!() };
        
        self.consume(Token::LeftParen, "Expect '(' after function name.")?;
        let mut params = Vec::new();
        if !self.check(Token::RightParen) {
            loop {
                let p_name_token = self.consume_identifier("Expect parameter name.")?;
                let p_name = if let Token::Identifier(s) = p_name_token { s } else { unreachable!() };
                
                let mut type_annotation = None;
                if self.matches(Token::Colon)? {
                    type_annotation = Some(self.parse_type()?);
                }
                
                params.push(TypedParam { name: p_name, type_annotation });
                if !self.matches(Token::Comma)? { break; }
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
        let name = if let Token::Identifier(s) = name_token { s } else { unreachable!() };
        
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

    // --- Statements ---

    fn statement(&mut self) -> PulseResult<Stmt> {
        if self.matches(Token::Print)? {
            self.print_statement()
        } else if self.matches(Token::If)? {
            self.if_statement()
        } else if self.matches(Token::While)? {
            self.while_statement()
        } else if self.matches(Token::Return)? {
            self.return_statement()
        } else if self.matches(Token::LeftBrace)? {
            Ok(Stmt::Block(self.block()?))
        } else {
            self.expression_statement()
        }
    }

    fn block(&mut self) -> PulseResult<Vec<Stmt>> {
        let mut stmts = Vec::new();
        while !self.check(Token::RightBrace) && !self.is_at_end() {
            match self.declaration()? {
                Decl::Stmt(s) => stmts.push(s),
                Decl::Function(name, params, ret, body) => {
                    // Local function - convert to a Let binding with a Closure?
                    // For now, let's just use Expression(Closure) wrapped in pseudo-Let if possible.
                    // Actually, let's just error for now to stay safe.
                    return Err(PulseError::CompileError("Local functions not yet supported in ParserV2 blocks".into(), self.lexer.line));
                },
                _ => return Err(PulseError::CompileError("Unsupported declaration in block".into(), self.lexer.line)),
            }
        }
        self.consume(Token::RightBrace, "Expect '}' after block.")?;
        Ok(stmts)
    }

    fn print_statement(&mut self) -> PulseResult<Stmt> {
        let expr = self.expression()?;
        self.consume(Token::Semicolon, "Expect ';' after value.")?;
        Ok(Stmt::Print(expr))
    }

    fn if_statement(&mut self) -> PulseResult<Stmt> {
        let condition = self.expression()?;
        let then_branch = Box::new(self.statement()?);
        let mut else_branch = None;
        if self.matches(Token::Else)? {
            else_branch = Some(Box::new(self.statement()?));
        }
        Ok(Stmt::If(condition, then_branch, else_branch))
    }

    fn while_statement(&mut self) -> PulseResult<Stmt> {
        let condition = self.expression()?;
        let body = Box::new(self.statement()?);
        Ok(Stmt::While(condition, body))
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
            },
            Token::Minus => {
                let expr = self.parse_precedence(PrecedenceV2::Unary)?;
                Ok(Expr::Unary(UnOp::Neg, Box::new(expr)))
            },
            Token::LeftParen => {
                let expr = self.expression()?;
                self.consume(Token::RightParen, "Expect ')' after expression.")?;
                Ok(expr)
            },
            _ => Err(PulseError::CompileError(format!("Expect expression, got {:?}", token), self.lexer.line)),
        }
    }

    fn infix_rule(&mut self, left: Expr, token: Token) -> PulseResult<Expr> {
        match token {
            Token::Plus | Token::Minus | Token::Star | Token::Slash |
            Token::EqualEqual | Token::BangEqual |
            Token::Less | Token::LessEqual | Token::Greater | Token::GreaterEqual |
            Token::And | Token::Or => {
                let op = self.token_to_binop(&token);
                let precedence = self.get_precedence(&token).next();
                let right = self.parse_precedence(precedence)?;
                Ok(Expr::Binary(Box::new(left), op, Box::new(right)))
            },
            Token::LeftParen => {
                let mut args = Vec::new();
                if !self.check(Token::RightParen) {
                    loop {
                        args.push(self.expression()?);
                        if !self.matches(Token::Comma)? { break; }
                    }
                }
                self.consume(Token::RightParen, "Expect ')' after arguments.")?;
                Ok(Expr::Call(Box::new(left), args))
            },
            _ => Err(PulseError::CompileError(format!("Unexpected infix token {:?}", token), self.lexer.line)),
        }
    }

    fn token_to_binop(&self, token: &Token) -> BinOp {
        match token {
            Token::Plus => BinOp::Add,
            Token::Minus => BinOp::Sub,
            Token::Star => BinOp::Mul,
            Token::Slash => BinOp::Div,
            Token::EqualEqual => BinOp::Eq,
            Token::BangEqual => BinOp::Ne,
            Token::Less => BinOp::Lt,
            Token::LessEqual => BinOp::Le,
            Token::Greater => BinOp::Gt,
            Token::GreaterEqual => BinOp::Ge,
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
            Token::Less | Token::LessEqual | Token::Greater | Token::GreaterEqual => PrecedenceV2::Comparison,
            Token::Plus | Token::Minus => PrecedenceV2::Term,
            Token::Star | Token::Slash => PrecedenceV2::Factor,
            Token::LeftParen => PrecedenceV2::Call,
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
            _ => Err(PulseError::CompileError(message.to_string(), self.lexer.line)),
        }
    }

    fn parse_type(&mut self) -> PulseResult<Type> {
        let token = self.advance_and_return()?;
        match token {
            Token::Identifier(s) => {
                match s.as_str() {
                    "Int" => Ok(Type::Int),
                    "Float" => Ok(Type::Float),
                    "Bool" => Ok(Type::Bool),
                    "String" => Ok(Type::String),
                    "Unit" => Ok(Type::Unit),
                    _ => Ok(Type::Custom(s)),
                }
            },
            _ => Err(PulseError::CompileError("Expect type name.".into(), self.lexer.line)),
        }
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
