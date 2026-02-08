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
    True, False, Nil, And, Or, Receive, Break, Continue,

    Eof,
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
                None => return Ok(Token::Eof),
                Some(c) => return Err(PulseError::IoError(format!("Unexpected character: {}", c))),
            }
        }
    }

    fn string(&mut self) -> Result<Token, PulseError> {
        let mut s = String::new();
        self.advance(); // Skip opening quote
        loop {
            match self.current {
                Some('"') => {
                    self.advance();
                    break;
                }
                Some(c) => {
                    s.push(c);
                    self.advance();
                }
                None => return Err(PulseError::IoError("Unterminated string".into())),
            }
        }
        Ok(Token::String(s))
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
            _ => Ok(Token::Identifier(s)),
        }
    }
}
