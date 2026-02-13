use serde::{Serialize, Deserialize, Serializer, Deserializer};
use crate::error::{PulseError, PulseResult};
use crate::object::{ObjHandle, HeapInterface, Function};
use std::sync::Arc;

// Wrappers for non-serializable types
#[derive(Clone, Debug)]
pub struct PulseSharedMemory(pub Arc<std::sync::Mutex<Vec<u8>>>);

impl PartialEq for PulseSharedMemory {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Serialize for PulseSharedMemory {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.serialize_unit()
    }
}

impl<'de> Deserialize<'de> for PulseSharedMemory {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        Err(serde::de::Error::custom("Cannot deserialize SharedMemory"))
    }
}


#[derive(Clone, Debug)]
pub struct PulseSocket(pub Arc<tokio::sync::Mutex<tokio::net::TcpStream>>);

impl PartialEq for PulseSocket {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Serialize for PulseSocket {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.serialize_unit()
    }
}

impl<'de> Deserialize<'de> for PulseSocket {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
         Err(serde::de::Error::custom("Cannot deserialize Socket"))
    }
}

#[derive(Clone, Debug)]
pub struct PulseListener(pub Arc<tokio::net::TcpListener>);

impl PartialEq for PulseListener {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Serialize for PulseListener {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.serialize_unit()
    }
}

impl<'de> Deserialize<'de> for PulseListener {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
         Err(serde::de::Error::custom("Cannot deserialize Listener"))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)] 
pub enum Constant {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Unit,
    SharedMemory(PulseSharedMemory),
    Socket(PulseSocket),
    Listener(PulseListener),
    Function(Box<Function>),
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
    pub node_id: u64,
    pub sequence: u32,
}

impl ActorId {
    pub fn new(node_id: u64, sequence: u32) -> Self {
        Self { node_id, sequence }
    }
}



use std::pin::Pin;
use std::future::Future;


pub type SyncNativeFn = fn(&mut dyn HeapInterface, &[Value]) -> PulseResult<Value>;
pub type AsyncNativeFn = for<'a> fn(&'a mut dyn HeapInterface, &'a [Value]) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>>;

#[derive(Clone)]
pub enum NativeFunctionKind {
    Sync(SyncNativeFn),
    Async(AsyncNativeFn),
}

#[derive(Clone)]
pub struct NativeFn {
    pub name: String,
    pub func: NativeFunctionKind,
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
            Constant::Function(_) => panic!("Cannot convert Function Constant to Value without Heap"),
            Constant::SharedMemory(_) | Constant::Socket(_) | Constant::Listener(_) => panic!("Cannot convert complex Constant to Value without Heap"),
        }
    }
}
