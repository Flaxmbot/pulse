use crate::value::ActorId;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Note,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticSpan {
    pub line: usize,
    pub column: Option<usize>,
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticFix {
    pub message: String,
    pub replacement: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub span: Option<DiagnosticSpan>,
    pub fixes: Vec<DiagnosticFix>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PulseError {
    CompileError(String, usize),
    RuntimeError(String),
    TypeMismatch {
        expected: String,
        got: String,
    },
    DivisionByZero,
    IndexOutOfBounds {
        index: i64,
        length: i64,
    },
    ArityMismatch {
        expected: usize,
        got: usize,
        name: String,
    },
    StackUnderflow,
    StackOverflow,
    UndefinedVariable(String),
    IoError(String),
    ActorNotFound(ActorId),
    InternalError(String),
    // Security errors
    PathTraversalAttempted(String),
    ImportPathNotAllowed(String),
    CapabilityDenied(String),
    ResourceLimitExceeded(String),
    MailboxFull,
    MessageTooLarge {
        size: usize,
        max: usize,
    },
}

impl PulseError {
    pub fn code(&self) -> &'static str {
        match self {
            PulseError::CompileError(_, _) => "PUL-E0001",
            PulseError::RuntimeError(_) => "PUL-E0002",
            PulseError::TypeMismatch { .. } => "PUL-E0003",
            PulseError::DivisionByZero => "PUL-E0004",
            PulseError::IndexOutOfBounds { .. } => "PUL-E0005",
            PulseError::ArityMismatch { .. } => "PUL-E0006",
            PulseError::StackUnderflow => "PUL-E0007",
            PulseError::StackOverflow => "PUL-E0008",
            PulseError::UndefinedVariable(_) => "PUL-E0009",
            PulseError::IoError(_) => "PUL-E0010",
            PulseError::ActorNotFound(_) => "PUL-E0011",
            PulseError::InternalError(_) => "PUL-E0012",
            PulseError::PathTraversalAttempted(_) => "PUL-E0100",
            PulseError::ImportPathNotAllowed(_) => "PUL-E0101",
            PulseError::CapabilityDenied(_) => "PUL-E0102",
            PulseError::ResourceLimitExceeded(_) => "PUL-E0103",
            PulseError::MailboxFull => "PUL-E0104",
            PulseError::MessageTooLarge { .. } => "PUL-E0105",
        }
    }

    pub fn to_diagnostic(&self) -> Diagnostic {
        let mut fixes = Vec::new();
        match self {
            PulseError::PathTraversalAttempted(_) => fixes.push(DiagnosticFix {
                message:
                    "Use a relative path under './', './lib', './vendor' or 'std/' without '..'."
                        .to_string(),
                replacement: None,
            }),
            PulseError::ImportPathNotAllowed(_) => fixes.push(DiagnosticFix {
                message:
                    "Move modules into an allowed import prefix or update compiler import policy."
                        .to_string(),
                replacement: None,
            }),
            PulseError::UndefinedVariable(name) => fixes.push(DiagnosticFix {
                message: format!(
                    "Declare '{}' before use or import the module that defines it.",
                    name
                ),
                replacement: None,
            }),
            PulseError::DivisionByZero => fixes.push(DiagnosticFix {
                message: "Guard divisors with a non-zero condition before division.".to_string(),
                replacement: None,
            }),
            PulseError::IndexOutOfBounds { index, length } => fixes.push(DiagnosticFix {
                message: format!(
                    "Index {} is outside [0, {}). Check bounds before indexing.",
                    index, length
                ),
                replacement: None,
            }),
            PulseError::ArityMismatch {
                expected,
                got,
                name,
            } => fixes.push(DiagnosticFix {
                message: format!(
                    "Call '{}' with {} argument(s); received {}.",
                    name, expected, got
                ),
                replacement: None,
            }),
            PulseError::ActorNotFound(id) => fixes.push(DiagnosticFix {
                message: format!(
                    "Ensure actor {:?} is spawned/registered before send/link/monitor.",
                    id
                ),
                replacement: None,
            }),
            PulseError::MailboxFull => fixes.push(DiagnosticFix {
                message:
                    "Increase mailbox capacity or add backpressure/retry logic before sending."
                        .to_string(),
                replacement: None,
            }),
            PulseError::MessageTooLarge { size, max } => fixes.push(DiagnosticFix {
                message: format!(
                    "Reduce payload size ({} bytes) to <= {} bytes or send a reference.",
                    size, max
                ),
                replacement: None,
            }),
            _ => {}
        }

        let span = self.primary_span();
        Diagnostic {
            code: self.code().to_string(),
            severity: DiagnosticSeverity::Error,
            message: self.to_string(),
            span,
            fixes,
        }
    }

    fn primary_span(&self) -> Option<DiagnosticSpan> {
        match self {
            PulseError::CompileError(message, line) => {
                let column = parse_compile_column(message);
                Some(DiagnosticSpan {
                    line: (*line).max(1),
                    column,
                    end_line: None,
                    end_column: None,
                })
            }
            _ => None,
        }
    }
}

fn parse_compile_column(message: &str) -> Option<usize> {
    let marker = "(at ";
    let start = message.find(marker)?;
    let rest = &message[start + marker.len()..];
    let end = rest.find(')')?;
    let loc = &rest[..end];
    let mut parts = loc.split(':');
    let _line = parts.next()?;
    let col = parts.next()?;
    col.parse::<usize>().ok()
}

impl fmt::Display for PulseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PulseError::CompileError(msg, line) => {
                write!(f, "Compile Error at line {}: {}", line, msg)
            }
            PulseError::RuntimeError(msg) => write!(f, "Runtime Error: {}", msg),
            PulseError::TypeMismatch { expected, got } => {
                write!(f, "Type mismatch: expected {}, got {}", expected, got)
            }
            PulseError::DivisionByZero => write!(f, "Division by zero"),
            PulseError::IndexOutOfBounds { index, length } => {
                write!(f, "Index {} out of bounds for length {}", index, length)
            }
            PulseError::ArityMismatch {
                expected,
                got,
                name,
            } => write!(
                f,
                "Function '{}' expects {} arguments, got {}",
                name, expected, got
            ),
            PulseError::StackUnderflow => write!(f, "Stack underflow"),
            PulseError::StackOverflow => write!(f, "Stack overflow"),
            PulseError::UndefinedVariable(name) => write!(f, "Undefined variable: {}", name),
            PulseError::IoError(msg) => write!(f, "IO Error: {}", msg),
            PulseError::ActorNotFound(id) => write!(f, "Actor not found: {:?}", id),
            PulseError::InternalError(msg) => write!(f, "VM Internal Error: {}", msg),
            PulseError::PathTraversalAttempted(path) => write!(
                f,
                "Security Error: Path traversal attempt detected in path '{}'",
                path
            ),
            PulseError::ImportPathNotAllowed(path) => write!(
                f,
                "Security Error: Import path '{}' is not in the allowed whitelist",
                path
            ),
            PulseError::CapabilityDenied(cap) => {
                write!(f, "Security Error: Capability '{}' denied", cap)
            }
            PulseError::ResourceLimitExceeded(msg) => write!(f, "Resource Limit Exceeded: {}", msg),
            PulseError::MailboxFull => write!(f, "Mailbox Error: Mailbox is full"),
            PulseError::MessageTooLarge { size, max } => write!(
                f,
                "Message Error: Message size {} exceeds maximum allowed {}",
                size, max
            ),
        }
    }
}

impl std::error::Error for PulseError {}

pub type PulseResult<T> = Result<T, PulseError>;
