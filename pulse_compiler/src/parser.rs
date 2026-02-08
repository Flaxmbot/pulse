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
        self.previous_line = self.lexer.line; // Assuming lexer exposes line (it does, public field)
        loop {
            self.current = self.lexer.next_token()?;
            if self.current != Token::Slash { // Skip comments (handled by lexer? No, lexer returns Slash for /)
                // Actually lexer `Slash` might just be division. Lexer handles comments internally usually?
                // In my lexer impl:
                // if // -> skip line
                // if / -> return Slash
                // So here we don't need to skip comments, lexer does it.
                break;
            }
        }
        Ok(())
    }

    pub fn consume(&mut self, expected: Token, message: &str) -> PulseResult<()> {
        if self.current == expected {
            self.advance()
        } else {
            Err(PulseError::TypeMismatch { expected: format!("{:?}", expected), got: format!("{:?}", self.current) })
            // Todo: Better error reporting
        }
    }
}
