use crate::lexer::{Lexer, Token};
use pulse_core::{PulseError, PulseResult};

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    pub current: Token,
    pub previous: Token,
    pub previous_line: usize,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            lexer: Lexer::new(source),
            current: Token::Eof,
            previous: Token::Eof,
            previous_line: 0,
        }
    }

    pub fn advance(&mut self) -> PulseResult<()> {
        self.previous = self.current.clone();
        self.previous_line = self.lexer.line;
        self.current = self.lexer.next_token()?;
        Ok(())
    }

    pub fn consume(&mut self, expected: Token, message: &str) -> PulseResult<()> {
        if self.current == expected {
            self.advance()
        } else {
            Err(PulseError::CompileError(message.to_string(), self.lexer.line))
        }
    }

    pub fn line(&self) -> usize {
        self.lexer.line
    }
}
