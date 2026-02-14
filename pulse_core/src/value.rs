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

// Wrapper enum for serializable constants only (excludes runtime handles)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SerializableConstant {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Unit,
    Function(Function),
}

impl From<Constant> for Option<SerializableConstant> {
    fn from(c: Constant) -> Self {
        match c {
            Constant::Bool(b) => Some(SerializableConstant::Bool(b)),
            Constant::Int(i) => Some(SerializableConstant::Int(i)),
            Constant::Float(f) => Some(SerializableConstant::Float(f)),
            Constant::String(s) => Some(SerializableConstant::String(s)),
            Constant::Unit => Some(SerializableConstant::Unit),
            Constant::Function(f) => Some(SerializableConstant::Function(*f)),
            _ => None, // Runtime handles cannot be serialized
        }
    }
}

impl From<SerializableConstant> for Constant {
    fn from(c: SerializableConstant) -> Self {
        match c {
            SerializableConstant::Bool(b) => Constant::Bool(b),
            SerializableConstant::Int(i) => Constant::Int(i),
            SerializableConstant::Float(f) => Constant::Float(f),
            SerializableConstant::String(s) => Constant::String(s),
            SerializableConstant::Unit => Constant::Unit,
            SerializableConstant::Function(f) => Constant::Function(Box::new(f)),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
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

impl Serialize for Constant {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        use serde::ser::SerializeStruct;
        
        match self {
            Constant::Bool(b) => {
                let mut s = serializer.serialize_struct("Constant", 2)?;
                s.serialize_field("type", "Bool")?;
                s.serialize_field("value", b)?;
                s.end()
            },
            Constant::Int(i) => {
                let mut s = serializer.serialize_struct("Constant", 2)?;
                s.serialize_field("type", "Int")?;
                s.serialize_field("value", i)?;
                s.end()
            },
            Constant::Float(f) => {
                let mut s = serializer.serialize_struct("Constant", 2)?;
                s.serialize_field("type", "Float")?;
                s.serialize_field("value", f)?;
                s.end()
            },
            Constant::String(s) => {
                let mut ser = serializer.serialize_struct("Constant", 2)?;
                ser.serialize_field("type", "String")?;
                ser.serialize_field("value", s)?;
                ser.end()
            },
            Constant::Unit => {
                let mut s = serializer.serialize_struct("Constant", 1)?;
                s.serialize_field("type", "Unit")?;
                s.end()
            },
            Constant::SharedMemory(_) => {
                Err(serde::ser::Error::custom("Cannot serialize SharedMemory - runtime reference"))
            },
            Constant::Socket(_) => {
                Err(serde::ser::Error::custom("Cannot serialize Socket - runtime reference"))
            },
            Constant::Listener(_) => {
                Err(serde::ser::Error::custom("Cannot serialize Listener - runtime reference"))
            },
            Constant::Function(func) => {
                let mut s = serializer.serialize_struct("Constant", 2)?;
                s.serialize_field("type", "Function")?;
                s.serialize_field("value", func.as_ref())?;
                s.end()
            },
        }
    }
}

impl<'de> Deserialize<'de> for Constant {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        use serde::de::Visitor;
        use std::fmt;
        
        struct ConstantVisitor;
        
        impl<'de> Visitor<'de> for ConstantVisitor {
            type Value = Constant;
            
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid Constant")
            }
            
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where A: serde::de::MapAccess<'de> {
                let mut type_val: Option<String> = None;
                
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "type" => {
                            type_val = Some(map.next_value()?);
                        },
                        "value" => {
                            let const_type = type_val.ok_or_else(|| serde::de::Error::custom("Missing type field"))?;
                            return match const_type.as_str() {
                                "Bool" => {
                                    let val: bool = map.next_value()?;
                                    Ok(Constant::Bool(val))
                                },
                                "Int" => {
                                    let val: i64 = map.next_value()?;
                                    Ok(Constant::Int(val))
                                },
                                "Float" => {
                                    let val: f64 = map.next_value()?;
                                    Ok(Constant::Float(val))
                                },
                                "String" => {
                                    let val: String = map.next_value()?;
                                    Ok(Constant::String(val))
                                },
                                "Unit" => Ok(Constant::Unit),
                                "Function" => {
                                    let func: Function = map.next_value()?;
                                    Ok(Constant::Function(Box::new(func)))
                                },
                                _ => Err(serde::de::Error::custom(format!("Unknown Constant type: {}", const_type))),
                            };
                        },
                        _ => return Err(serde::de::Error::custom("Unknown field")),
                    }
                }
                Err(serde::de::Error::custom("Missing value field"))
            }
        }
        
        deserializer.deserialize_map(ConstantVisitor)
    }
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

    pub fn as_float(&self) -> PulseResult<f64> {
        match self {
            Value::Float(f) => Ok(*f),
            Value::Int(i) => Ok(*i as f64),
            _ => Err(PulseError::TypeMismatch { expected: "float".into(), got: self.type_name() }),
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
