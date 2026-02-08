use serde::{Serialize, Deserialize};
use crate::error::{PulseError, PulseResult};
use crate::object::{ObjHandle, HeapInterface, Function};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)] 
pub enum Constant {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Unit,
    Function(Box<Function>), // Boxed to avoid infinite size if Function was inline (struct contains Chunk contains Vec<Constant>)
    // Actually Chunk contains Vec, so indirection exists. But Function is struct.
    // Constant -> Function -> Chunk -> Vec<Constant>.
    // Vec is pointer. So Function size is fixed.
    // Constant size is max(size of variants).
    // So explicit Box not strictly necessary for size, but good for move?
    // Let's use Box just in case or simpler: `Function` struct.
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Unit,
    Obj(ObjHandle),
    Pid(ActorId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActorId {
    pub node_id: u128,
    pub sequence: u64,
}

impl ActorId {
    pub fn new(node_id: u128, sequence: u64) -> Self {
        Self { node_id, sequence }
    }
}

// NativeFn definition. 
// Note: It is NOT stored in Value anymore, but in Object.
#[derive(Clone)]
pub struct NativeFn {
    pub name: String,
    pub func: fn(&mut dyn HeapInterface, &[Value]) -> PulseResult<Value>,
}

impl std::fmt::Debug for NativeFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<native fn {}>", self.name)
    }
}

impl PartialEq for NativeFn {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name 
    }
}

impl Value {
    pub fn type_name(&self) -> String {
        match self {
            Value::Bool(_) => "bool".to_string(),
            Value::Int(_) => "int".to_string(),
            Value::Float(_) => "float".to_string(),
            Value::Unit => "unit".to_string(),
            Value::Obj(_) => "object".to_string(), 
            Value::Pid(_) => "pid".to_string(),
        }
    }

    pub fn as_int(&self) -> PulseResult<i64> {
        match self {
            Value::Int(i) => Ok(*i),
            _ => Err(PulseError::TypeMismatch { expected: "int".into(), got: self.type_name() }),
        }
    }

    pub fn as_bool(&self) -> PulseResult<bool> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err(PulseError::TypeMismatch { expected: "bool".into(), got: self.type_name() }),
        }
    }
}

impl From<Constant> for Value {
    fn from(c: Constant) -> Self {
        match c {
            Constant::Bool(b) => Value::Bool(b),
            Constant::Int(i) => Value::Int(i),
            Constant::Float(f) => Value::Float(f),
            Constant::Unit => Value::Unit,
            Constant::String(_) => panic!("Cannot convert String Constant to Value without Heap"),
            Constant::Function(_) => panic!("Cannot convert Function Constant to Value without Heap (needs Closure wrapper)"),
        }
    }
}
