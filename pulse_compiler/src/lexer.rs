use pulse_core::PulseError;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Single-char
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    Percent,
    Ampersand,
    Colon,
    Underscore,
    Caret,
    Tilde,
    LeftAngle,
    RightAngle, // < > for generic types

    // One or two char
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    LogicalAnd,
    LogicalOr,
    StarStar,
    ShiftLeft,
    ShiftRight,
    DotDot,
    PlusEqual,
    MinusEqual,
    StarEqual,
    SlashEqual,
    PercentEqual,

    // Literals
    Identifier(String),
    String(String),
    Int(i64),
    Float(f64),

    // Keywords
    Actor,
    On,
    Message,
    Send,
    Spawn,
    Fn,
    Def,
    Let,
    If,
    Else,
    While,
    For,
    In,
    Return,
    Print,
    True,
    False,
    Nil,
    And,
    Or,
    Receive,
    Break,
    Continue,
    Const,
    Import,
    As,
    Link,
    Monitor,
    SpawnLink,
    Register,
    Unregister,
    WhereIs,
    Match,
    FatArrow,
    Pipe,
    Arrow,
    Try,
    Catch,
    Throw,
    Test,
    DocComment(String),
    // Class/Object keywords
    Class,
    Extends,
    Super,
    This,
    // Type keywords
    TypeInt,
    TypeFloat,
    TypeBool,
    TypeString,
    TypeUnit,
    TypePid,
    TypeList,
    TypeMap,
    TypeFn,
    TypeAny,
    TypeAtomic,
    // Type guard keyword
    Is,
    // Shared Memory keywords
    Shared,
    Memory,
    Lock,
    Unlock,
    // Atomic keywords
    Atomic,
    // Memory Fence keywords
    Fence,
    Acquire,
    Release,

    Error,
    // Interpolated String Parts
    InterpolatedString(Vec<StringPart>),

    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    Literal(String),
    Expr(String), // The expression source code inside ${}
}

#[derive(Clone)]
pub struct Lexer<'a> {
    #[allow(dead_code)]
    source: &'a str,
    chars: std::str::Chars<'a>,
    current: Option<char>,
    pub line: usize,
    pub column: usize,
    pub token_line: usize,
    pub token_column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut l = Self {
            source,
            chars: source.chars(),
            current: None,
            line: 1,
            column: 1,
            token_line: 1,
            token_column: 1,
        };
        l.advance();
        l
    }

    fn advance(&mut self) {
        if let Some(prev) = self.current {
            if prev == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        self.current = self.chars.next();
    }

    pub fn next_token(&mut self) -> Result<Token, PulseError> {
        loop {
            self.token_line = self.line;
            self.token_column = self.column;
            match self.current {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                Some('/') => {
                    self.advance();
                    if self.current == Some('/') {
                        self.advance();
                        // Check for doc comment ///
                        if self.current == Some('/') {
                            self.advance();
                            let mut doc = String::new();
                            while let Some(ch) = self.current {
                                if ch == '\n' {
                                    break;
                                }
                                doc.push(ch);
                                self.advance();
                            }
                            return Ok(Token::DocComment(doc.trim().to_string()));
                        } else {
                            // Regular comment
                            while self.current != Some('\n') && self.current.is_some() {
                                self.advance();
                            }
                        }
                    } else if self.current == Some('*') {
                        // Block comment - check for /** doc */
                        self.advance();
                        let is_doc = self.current == Some('*');
                        let mut doc = String::new();
                        loop {
                            if self.current.is_none() {
                                break;
                            }
                            if self.current == Some('*') {
                                self.advance();
                                if self.current == Some('/') {
                                    self.advance();
                                    break;
                                }
                                if is_doc {
                                    doc.push('*');
                                }
                            } else {
                                if is_doc {
                                    doc.push(self.current.clone().expect("Expected a value"));
                                }
                                self.advance();
                            }
                        }
                        if is_doc {
                            return Ok(Token::DocComment(doc.trim().to_string()));
                        }
                    } else if self.current == Some('=') {
                        self.advance();
                        return Ok(Token::SlashEqual);
                    } else {
                        return Ok(Token::Slash);
                    }
                }
                Some('(') => {
                    self.advance();
                    return Ok(Token::LeftParen);
                }
                Some(')') => {
                    self.advance();
                    return Ok(Token::RightParen);
                }
                Some('{') => {
                    self.advance();
                    return Ok(Token::LeftBrace);
                }
                Some('}') => {
                    self.advance();
                    return Ok(Token::RightBrace);
                }

                Some('[') => {
                    self.advance();
                    return Ok(Token::LeftBracket);
                }
                Some(']') => {
                    self.advance();
                    return Ok(Token::RightBracket);
                }
                Some(':') => {
                    self.advance();
                    return Ok(Token::Colon);
                }
                Some('_') => {
                    // Consume the underscore and see what's next
                    self.advance();
                    // Check if next character is alphanumeric or underscore
                    if let Some(c) = self.current {
                        if c.is_alphanumeric() || c == '_' {
                            // Continue parsing identifier
                            let mut s = String::from('_');
                            while let Some(ch) = self.current {
                                if ch.is_alphanumeric() || ch == '_' {
                                    s.push(ch);
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                            match s.as_str() {
                                _ => return Ok(Token::Identifier(s)),
                            }
                        }
                    }
                    // If next character isn't valid for identifier, return Underscore token
                    return Ok(Token::Underscore);
                }
                Some(',') => {
                    self.advance();
                    return Ok(Token::Comma);
                }
                Some('.') => {
                    self.advance();
                    if self.current == Some('.') {
                        self.advance();
                        return Ok(Token::DotDot);
                    }
                    return Ok(Token::Dot);
                }
                Some('-') => {
                    self.advance();
                    if self.current == Some('>') {
                        self.advance();
                        return Ok(Token::Arrow);
                    }
                    if self.current == Some('=') {
                        self.advance();
                        return Ok(Token::MinusEqual);
                    }
                    return Ok(Token::Minus);
                }
                Some('+') => {
                    self.advance();
                    if self.current == Some('=') {
                        self.advance();
                        return Ok(Token::PlusEqual);
                    }
                    return Ok(Token::Plus);
                }
                Some(';') => {
                    self.advance();
                    return Ok(Token::Semicolon);
                }
                Some('*') => {
                    self.advance();
                    if self.current == Some('*') {
                        self.advance();
                        return Ok(Token::StarStar);
                    }
                    if self.current == Some('=') {
                        self.advance();
                        return Ok(Token::StarEqual);
                    }
                    return Ok(Token::Star);
                }
                Some('%') => {
                    self.advance();
                    if self.current == Some('=') {
                        self.advance();
                        return Ok(Token::PercentEqual);
                    }
                    return Ok(Token::Percent);
                }
                Some('!') => {
                    self.advance();
                    if self.current == Some('=') {
                        self.advance();
                        return Ok(Token::BangEqual);
                    }
                    return Ok(Token::Bang);
                }
                Some('&') => {
                    self.advance();
                    if self.current == Some('&') {
                        self.advance();
                        return Ok(Token::LogicalAnd);
                    }
                    return Ok(Token::Ampersand);
                }
                Some('|') => {
                    self.advance();
                    if self.current == Some('|') {
                        self.advance();
                        return Ok(Token::LogicalOr);
                    }
                    return Ok(Token::Pipe);
                }
                Some('=') => {
                    self.advance();
                    if self.current == Some('=') {
                        self.advance();
                        return Ok(Token::EqualEqual);
                    }
                    if self.current == Some('>') {
                        self.advance();
                        return Ok(Token::FatArrow);
                    }
                    return Ok(Token::Equal);
                }
                Some('<') => {
                    self.advance();
                    if self.current == Some('=') {
                        self.advance();
                        return Ok(Token::LessEqual);
                    }
                    if self.current == Some('<') {
                        self.advance();
                        return Ok(Token::ShiftLeft);
                    }
                    return Ok(Token::LeftAngle);
                }
                Some('>') => {
                    self.advance();
                    if self.current == Some('=') {
                        self.advance();
                        return Ok(Token::GreaterEqual);
                    }
                    if self.current == Some('>') {
                        self.advance();
                        return Ok(Token::ShiftRight);
                    }
                    return Ok(Token::RightAngle);
                }
                Some('^') => {
                    self.advance();
                    return Ok(Token::Caret);
                }
                Some('~') => {
                    self.advance();
                    return Ok(Token::Tilde);
                }
                Some('"') => return self.string(),
                Some(c) if c.is_ascii_digit() => return self.number(),
                Some(c) if c.is_alphabetic() || c == '_' => return self.identifier(),
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
                                Some('{') => {
                                    brace_depth += 1;
                                    expr.push('{');
                                    self.advance();
                                }
                                Some('}') => {
                                    brace_depth -= 1;
                                    if brace_depth > 0 {
                                        expr.push('}');
                                    }
                                    self.advance();
                                }
                                Some(c) => {
                                    expr.push(c);
                                    self.advance();
                                }
                                None => {
                                    return Err(PulseError::IoError(
                                        "Unterminated interpolation".into(),
                                    ))
                                }
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
                        Some('n') => {
                            current_literal.push('\n');
                            self.advance();
                        }
                        Some('t') => {
                            current_literal.push('\t');
                            self.advance();
                        }
                        Some('r') => {
                            current_literal.push('\r');
                            self.advance();
                        }
                        Some('0') => {
                            current_literal.push('\0');
                            self.advance();
                        }
                        Some('\\') => {
                            current_literal.push('\\');
                            self.advance();
                        }
                        Some('"') => {
                            current_literal.push('"');
                            self.advance();
                        }
                        Some('$') => {
                            current_literal.push('$');
                            self.advance();
                        }
                        Some(c) => {
                            current_literal.push(c);
                            self.advance();
                        }
                        None => {
                            return Err(PulseError::IoError("Unterminated escape sequence".into()))
                        }
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
            if c.is_ascii_digit() {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }

        if self.current == Some('.') {
            let mut lookahead = self.chars.clone();
            let next = lookahead.next();

            // Parse float only when `.` is followed by a digit.
            // This preserves range syntax like `1..10` as Int + DotDot.
            if matches!(next, Some(c) if c.is_ascii_digit()) {
                s.push('.');
                self.advance();
                while let Some(c) = self.current {
                    if c.is_ascii_digit() {
                        s.push(c);
                        self.advance();
                    } else {
                        break;
                    }
                }
                let n: f64 = s
                    .parse()
                    .map_err(|_| PulseError::IoError("Invalid float".into()))?;
                Ok(Token::Float(n))
            } else {
                let n: i64 = s
                    .parse()
                    .map_err(|_| PulseError::IoError("Invalid integer".into()))?;
                Ok(Token::Int(n))
            }
        } else {
            let n: i64 = s
                .parse()
                .map_err(|_| PulseError::IoError("Invalid integer".into()))?;
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
            "as" => Ok(Token::As),
            "let" => Ok(Token::Let),
            "if" => Ok(Token::If),
            "else" => Ok(Token::Else),
            "while" => Ok(Token::While),
            "print" => Ok(Token::Print),
            "for" => Ok(Token::For),
            "break" => Ok(Token::Break),
            "continue" => Ok(Token::Continue),
            "const" => Ok(Token::Const),
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
            "test" => Ok(Token::Test),
            "def" => Ok(Token::Def),
            "in" => Ok(Token::In),
            "class" => Ok(Token::Class),
            "extends" => Ok(Token::Extends),
            "super" => Ok(Token::Super),
            "this" => Ok(Token::This),
            // Type keywords (capitalized for types)
            "Int" => Ok(Token::TypeInt),
            "Float" => Ok(Token::TypeFloat),
            "Bool" => Ok(Token::TypeBool),
            "String" => Ok(Token::TypeString),
            "Unit" => Ok(Token::TypeUnit),
            "Pid" => Ok(Token::TypePid),
            "List" => Ok(Token::TypeList),
            "Map" => Ok(Token::TypeMap),
            "Fn" => Ok(Token::TypeFn),
            "Any" => Ok(Token::TypeAny),
            "Atomic" => Ok(Token::TypeAtomic),
            // Type guard keyword
            "is" => Ok(Token::Is),
            // Shared memory keywords
            "shared" => Ok(Token::Shared),
            "memory" => Ok(Token::Memory),
            "lock" => Ok(Token::Lock),
            "unlock" => Ok(Token::Unlock),
            // Atomic keywords
            "atomic" => Ok(Token::Atomic),
            // Memory Fence keywords
            "fence" => Ok(Token::Fence),
            "acquire" => Ok(Token::Acquire),
            "release" => Ok(Token::Release),
            _ => Ok(Token::Identifier(s)),
        }
    }
}
