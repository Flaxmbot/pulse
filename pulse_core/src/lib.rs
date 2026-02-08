pub mod value;
pub mod bytecode;
pub mod error;
pub mod object;

pub use value::{Value, Constant, NativeFn, ActorId};
pub use object::{Object, ObjHandle, HeapInterface};
pub use error::{PulseError, PulseResult};
pub use bytecode::{Op, Chunk};
