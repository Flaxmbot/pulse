pub mod bytecode;
pub mod error;
pub mod object;
pub mod value;
// mod send_check;

pub use bytecode::{Chunk, Op};
pub use error::{
    Diagnostic, DiagnosticFix, DiagnosticSeverity, DiagnosticSpan, PulseError, PulseResult,
};
pub use object::{Function, HeapInterface, ObjHandle, Object};
pub use value::{ActorId, Constant, NativeFn, Value};
