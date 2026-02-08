use std::fmt;
use crate::value::ActorId;

#[derive(Debug, Clone, PartialEq)]
pub enum PulseError {
    CompileError(String, usize),
    RuntimeError(String),
    TypeMismatch { expected: String, got: String },
    StackUnderflow,
    StackOverflow,
    UndefinedVariable(String),
    IoError(String),
    ActorNotFound(ActorId),
}

impl fmt::Display for PulseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PulseError::CompileError(msg, line) => write!(f, "Compile Error at line {}: {}", line, msg),
            PulseError::RuntimeError(msg) => write!(f, "Runtime Error: {}", msg),
            PulseError::TypeMismatch { expected, got } => write!(f, "Type mismatch: expected {}, got {}", expected, got),
            PulseError::StackUnderflow => write!(f, "Stack underflow"),
            PulseError::StackOverflow => write!(f, "Stack overflow"),
            PulseError::UndefinedVariable(name) => write!(f, "Undefined variable: {}", name),
            PulseError::IoError(msg) => write!(f, "IO Error: {}", msg),
            PulseError::ActorNotFound(id) => write!(f, "Actor not found: {:?}", id),
        }
    }
}

impl std::error::Error for PulseError {}

pub type PulseResult<T> = Result<T, PulseError>;
