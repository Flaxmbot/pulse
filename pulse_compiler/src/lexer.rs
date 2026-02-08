use pulse_core::PulseError;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Single-char
    LeftParen, RightParen, LeftBrace, RightBrace, LeftBracket, RightBracket,
    Comma, Dot, Minus, Plus, Semicolon, Slash, Star, Colon,
    
    // One or two char
    Bang, BangEqual,
    Equal, EqualEqual,
    Greater, GreaterEqual,
    Less, LessEqual,
    
    // Literals
    Identifier(String),
    String(String),
    Int(i64),
    Float(f64),
    
    // Keywords
    Actor, On, Message, Send, Spawn, Fn, Let, If, Else, While, For, Return, Print,
    True, False, Nil, And, Or, Receive, Break, Continue, Import, Link, Monitor, SpawnLink,
    Register, Unregister, WhereIs,
    Match, FatArrow, Pipe,
    Try, Catch, Throw,

    // Interpolated String Parts
    InterpolatedString(Vec<StringPart>),

    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    Literal(String),
    Expr(String), // The expression source code inside ${}
}

pub struct Lexer<'a> {
    source: &'a str,
    chars: std::str::Chars<'a>,
    current: Option<char>,
    pub line: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut l = Self {
            source,
            chars: source.chars(),
            current: None,
            line: 1,
        };
        l.advance();
        l
    }

    fn advance(&mut self) {
        self.current = self.chars.next();
    }

    pub fn next_token(&mut self) -> Result<Token, PulseError> {
        loop {
            match self.current {
                Some(c) if c.is_whitespace() => {
                    if c == '\n' { self.line += 1; }
                    self.advance();
                }
                Some('/') => {
                    self.advance();
                    if self.current == Some('/') {
                        // Comment
                        while self.current != Some('\n') && self.current != None {
                            self.advance();
                        }
                    } else {
                        return Ok(Token::Slash);
                    }
                }
                Some('(') => { self.advance(); return Ok(Token::LeftParen); }
                Some(')') => { self.advance(); return Ok(Token::RightParen); }
                Some('{') => { self.advance(); return Ok(Token::LeftBrace); }
                Some('}') => { self.advance(); return Ok(Token::RightBrace); }

                Some('[') => { self.advance(); return Ok(Token::LeftBracket); }
                Some(']') => { self.advance(); return Ok(Token::RightBracket); }
                Some(':') => { self.advance(); return Ok(Token::Colon); }
                Some(',') => { self.advance(); return Ok(Token::Comma); }
                Some('.') => { self.advance(); return Ok(Token::Dot); }
                Some('-') => { self.advance(); return Ok(Token::Minus); }
                Some('+') => { self.advance(); return Ok(Token::Plus); }
                Some(';') => { self.advance(); return Ok(Token::Semicolon); }
                Some('*') => { self.advance(); return Ok(Token::Star); }
                Some('!') => {
                    self.advance();
                    if self.current == Some('=') { self.advance(); return Ok(Token::BangEqual); }
                    return Ok(Token::Bang);
                }
                Some('=') => {
                    self.advance();
                    if self.current == Some('=') { self.advance(); return Ok(Token::EqualEqual); }
                    if self.current == Some('>') { self.advance(); return Ok(Token::FatArrow); }
                    return Ok(Token::Equal);
                }
                Some('<') => {
                    self.advance();
                    if self.current == Some('=') { self.advance(); return Ok(Token::LessEqual); }
                    return Ok(Token::Less);
                }
                Some('>') => {
                    self.advance();
                    if self.current == Some('=') { self.advance(); return Ok(Token::GreaterEqual); }
                    return Ok(Token::Greater);
                }
                Some('"') => return self.string(),
                Some(c) if c.is_digit(10) => return self.number(),
                Some(c) if c.is_alphabetic() || c == '_' => return self.identifier(),
                Some('|') => { self.advance(); return Ok(Token::Pipe); }
                None => return Ok(Token::Eof),
                Some(c) => return Err(PulseError::IoError(format!("Unexpected character: {}", c))),
            }
        }
    }

    fn string(&mut self) -> Result<Token, PulseError> {
        let mut parts: Vec<StringPart> = Vec::new();
        let mut current_literal = String::new();
        self.advance(); // Skip opening quote
        
        loop {
            match self.current {
                Some('"') => {
                    self.advance();
                    break;
                }
                Some('$') => {
                    self.advance();
                    if self.current == Some('{') {
                        // Save current literal if any
                        if !current_literal.is_empty() {
                            parts.push(StringPart::Literal(current_literal.clone()));
                            current_literal.clear();
                        }
                        
                        self.advance(); // Skip '{'
                        let mut expr = String::new();
                        let mut brace_depth = 1;
                        
                        while brace_depth > 0 {
                            match self.current {
                                Some('{') => { brace_depth += 1; expr.push('{'); self.advance(); }
                                Some('}') => { 
                                    brace_depth -= 1; 
                                    if brace_depth > 0 { expr.push('}'); }
                                    self.advance(); 
                                }
                                Some(c) => { expr.push(c); self.advance(); }
                                None => return Err(PulseError::IoError("Unterminated interpolation".into())),
                            }
                        }
                        
                        parts.push(StringPart::Expr(expr));
                    } else {
                        current_literal.push('$');
                    }
                }
                Some('\\') => {
                    self.advance();
                    match self.current {
                        Some('n') => { current_literal.push('\n'); self.advance(); }
                        Some('t') => { current_literal.push('\t'); self.advance(); }
                        Some('\\') => { current_literal.push('\\'); self.advance(); }
                        Some('"') => { current_literal.push('"'); self.advance(); }
                        Some('$') => { current_literal.push('$'); self.advance(); }
                        Some(c) => { current_literal.push(c); self.advance(); }
                        None => return Err(PulseError::IoError("Unterminated escape sequence".into())),
                    }
                }
                Some(c) => {
                    current_literal.push(c);
                    self.advance();
                }
                None => return Err(PulseError::IoError("Unterminated string".into())),
            }
        }
        
        // Save remaining literal
        if !current_literal.is_empty() {
            parts.push(StringPart::Literal(current_literal));
        }
        
        // If no interpolation, return simple string
        if parts.len() == 1 {
            if let StringPart::Literal(s) = &parts[0] {
                return Ok(Token::String(s.clone()));
            }
        }
        
        if parts.is_empty() {
            return Ok(Token::String(String::new()));
        }
        
        Ok(Token::InterpolatedString(parts))
    }

    fn number(&mut self) -> Result<Token, PulseError> {
        let mut s = String::new();
        while let Some(c) = self.current {
            if c.is_digit(10) {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        
        if self.current == Some('.') {
             s.push('.');
             self.advance();
             while let Some(c) = self.current {
                if c.is_digit(10) {
                    s.push(c);
                    self.advance();
                } else {
                    break;
                }
             }
             let n: f64 = s.parse().map_err(|_| PulseError::IoError("Invalid float".into()))?;
             Ok(Token::Float(n))
        } else {
             let n: i64 = s.parse().map_err(|_| PulseError::IoError("Invalid integer".into()))?;
             Ok(Token::Int(n))
        }
    }

    fn identifier(&mut self) -> Result<Token, PulseError> {
        let mut s = String::new();
        while let Some(c) = self.current {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }

        match s.as_str() {
            "actor" => Ok(Token::Actor),
            "on" => Ok(Token::On),
            "message" => Ok(Token::Message),
            "send" => Ok(Token::Send),
            "spawn" => Ok(Token::Spawn),
            "fn" => Ok(Token::Fn),
            "let" => Ok(Token::Let),
            "if" => Ok(Token::If),
            "else" => Ok(Token::Else),
            "while" => Ok(Token::While),
            "print" => Ok(Token::Print),
            "for" => Ok(Token::For),
            "break" => Ok(Token::Break),
            "continue" => Ok(Token::Continue),
            "return" => Ok(Token::Return),
            "true" => Ok(Token::True),
            "false" => Ok(Token::False),
            "nil" => Ok(Token::Nil),
            "and" => Ok(Token::And),
            "or" => Ok(Token::Or),
            "receive" => Ok(Token::Receive),
            "import" => Ok(Token::Import),
            "link" => Ok(Token::Link),
            "monitor" => Ok(Token::Monitor),
            "spawn_link" => Ok(Token::SpawnLink),
            "register" => Ok(Token::Register),
            "unregister" => Ok(Token::Unregister),
            "whereis" => Ok(Token::WhereIs),
            "match" => Ok(Token::Match),
            "try" => Ok(Token::Try),
            "catch" => Ok(Token::Catch),
            "throw" => Ok(Token::Throw),
            _ => Ok(Token::Identifier(s)),
        }
    }
}
