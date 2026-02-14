use serde::{Serialize, Deserialize, Serializer, Deserializer};
use crate::value::{NativeFn, Value};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use regex::Regex;

pub use crate::value::PulseSocket;
pub use crate::value::PulseListener;
pub use crate::value::PulseSharedMemory;
use crate::bytecode::Chunk;
use std::collections::{HashMap, VecDeque};

/// Atomic integer for safe concurrent access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicInt {
    pub value: Arc<AtomicI64>,
}

impl AtomicInt {
    pub fn new(initial: i64) -> Self {
        Self {
            value: Arc::new(AtomicI64::new(initial)),
        }
    }
    
    pub fn load(&self, order: Ordering) -> i64 {
        self.value.load(order)
    }
    
    pub fn store(&self, val: i64, order: Ordering) {
        self.value.store(val, order);
    }
    
    pub fn fetch_add(&self, val: i64, order: Ordering) -> i64 {
        self.value.fetch_add(val, order)
    }
    
    pub fn fetch_sub(&self, val: i64, order: Ordering) -> i64 {
        self.value.fetch_sub(val, order)
    }
    
    pub fn compare_exchange(&self, current: i64, new: i64, success: Ordering, failure: Ordering) -> Result<i64, i64> {
        self.value.compare_exchange(current, new, success, failure)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjHandle(pub usize);

impl ObjHandle {
    /// Convert to a shared handle index (upper bit set indicates shared)
    pub fn to_shared_handle(&self) -> Option<usize> {
        // Use upper bit to indicate shared handle
        if self.0 & (1 << 63) != 0 {
            Some(self.0 & !(1 << 63))
        } else {
            None
        }
    }
    
    /// Create ObjHandle from a shared handle index
    pub fn from_shared_handle(idx: usize) -> Self {
        // Set upper bit to indicate this is a shared handle
        ObjHandle(idx | (1 << 63))
    }
    
    /// Check if this is a shared handle
    pub fn is_shared(&self) -> bool {
        self.0 & (1 << 63) != 0
    }
}

#[derive(Debug, Clone)]
pub struct SharedMemory {
    pub value: Value,
    pub locked: bool,
}

impl Serialize for SharedMemory {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        // Serialize only the locked state, value cannot be serialized
        serializer.serialize_str("SharedMemory")
    }
}

impl<'de> Deserialize<'de> for SharedMemory {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        Err(serde::de::Error::custom("Cannot deserialize SharedMemory - needs runtime allocation"))
    }
}

#[derive(Debug, Clone)]
pub struct BoundMethod {
    pub receiver: Value,
    pub method: Function, // Or Closure? Usually Closure.
}

impl Serialize for BoundMethod {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        // Cannot serialize bound method as it contains runtime Value
        Err(serde::ser::Error::custom("Cannot serialize BoundMethod - contains runtime references"))
    }
}

impl<'de> Deserialize<'de> for BoundMethod {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        Err(serde::de::Error::custom("Cannot deserialize BoundMethod - needs runtime allocation"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub class: Arc<Class>,
    pub fields: HashMap<String, Value>,
}

// PulseSocket moved to value.rs

#[derive(Debug, Clone)]
pub enum Object {
    String(String),
    NativeFn(NativeFn),
    Function(Function),
    Closure(Closure),
    Upvalue(Upvalue),
    List(Vec<Value>),
    Map(HashMap<String, Value>),
    Module(HashMap<String, Value>),
    Class(Class),
    Instance(Instance),
    BoundMethod(BoundMethod),
    Set(std::collections::HashSet<String>), // Using string representation for now to avoid hashing issues
    Queue(VecDeque<Value>),

    SharedMemory(SharedMemory),

    AtomicInt(AtomicInt),

    SharedBuffer(PulseSharedMemory),
    Socket(PulseSocket),
    Listener(PulseListener),
    
    /// Compiled regex - stored as Arc for cheap cloning
    Regex(Arc<Regex>),
}

// Custom serialization for Object enum that handles non-serializable variants
impl Serialize for Object {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        use serde::ser::SerializeStruct;
        
        match self {
            Object::String(str_val) => {
                let mut s = serializer.serialize_struct("Object", 2)?;
                s.serialize_field("type", "String")?;
                s.serialize_field("value", &str_val)?;
                s.end()
            },
            Object::NativeFn(nf) => {
                let mut s = serializer.serialize_struct("Object", 2)?;
                s.serialize_field("type", "NativeFn")?;
                s.serialize_field("name", &nf.name)?;
                s.end()
            },
            Object::Function(f) => {
                let mut s = serializer.serialize_struct("Object", 2)?;
                s.serialize_field("type", "Function")?;
                s.serialize_field("value", f)?;
                s.end()
            },
            Object::Closure(c) => {
                // Serialize closure as the underlying function (upvalues are runtime-specific)
                let mut s = serializer.serialize_struct("Object", 2)?;
                s.serialize_field("type", "Closure")?;
                s.serialize_field("function", &c.function)?;
                s.end()
            },
            Object::Upvalue(_) => {
                Err(serde::ser::Error::custom("Cannot serialize Upvalue - runtime handle"))
            },
            Object::List(list) => {
                let mut s = serializer.serialize_struct("Object", 2)?;
                s.serialize_field("type", "List")?;
                s.serialize_field("value", list)?;
                s.end()
            },
            Object::Map(map) => {
                let mut s = serializer.serialize_struct("Object", 2)?;
                s.serialize_field("type", "Map")?;
                s.serialize_field("value", map)?;
                s.end()
            },
            Object::Module(module) => {
                let mut s = serializer.serialize_struct("Object", 2)?;
                s.serialize_field("type", "Module")?;
                s.serialize_field("value", module)?;
                s.end()
            },
            Object::Class(class) => {
                let mut s = serializer.serialize_struct("Object", 2)?;
                s.serialize_field("type", "Class")?;
                s.serialize_field("value", class)?;
                s.end()
            },
            Object::Instance(instance) => {
                let mut s = serializer.serialize_struct("Object", 2)?;
                s.serialize_field("type", "Instance")?;
                s.serialize_field("class_name", &instance.class.name)?;
                s.serialize_field("fields", &instance.fields)?;
                s.end()
            },
            Object::BoundMethod(_) => {
                Err(serde::ser::Error::custom("Cannot serialize BoundMethod - runtime reference"))
            },
            Object::Set(set) => {
                let mut s = serializer.serialize_struct("Object", 2)?;
                s.serialize_field("type", "Set")?;
                s.serialize_field("value", set)?;
                s.end()
            },
            Object::Queue(queue) => {
                let mut s = serializer.serialize_struct("Object", 2)?;
                s.serialize_field("type", "Queue")?;
                s.serialize_field("value", queue)?;
                s.end()
            },
            Object::SharedMemory(_) => {
                Err(serde::ser::Error::custom("Cannot serialize SharedMemory - runtime reference"))
            },
            Object::AtomicInt(atomic) => {
                let mut s = serializer.serialize_struct("Object", 2)?;
                s.serialize_field("type", "AtomicInt")?;
                s.serialize_field("value", &atomic.value.load(Ordering::SeqCst))?;
                s.end()
            },
            Object::SharedBuffer(_) => {
                Err(serde::ser::Error::custom("Cannot serialize SharedBuffer - runtime reference"))
            },
            Object::Socket(_) => {
                Err(serde::ser::Error::custom("Cannot serialize Socket - runtime reference"))
            },
            Object::Listener(_) => {
                Err(serde::ser::Error::custom("Cannot serialize Listener - runtime reference"))
            },
            Object::Regex(_) => {
                Err(serde::ser::Error::custom("Cannot serialize Regex - contains compiled pattern"))
            },
        }
    }
}

impl<'de> Deserialize<'de> for Object {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        Err(serde::de::Error::custom("Cannot deserialize Object directly - use ObjectRef or specific deserialize methods"))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Function {
    pub arity: usize,
    pub chunk: Arc<Chunk>,
    pub name: String,
    pub upvalue_count: usize,
    pub module_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Closure {
    pub function: Function,
    pub upvalues: Vec<ObjHandle>,
}

impl Serialize for Closure {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        // Serialize closure as its underlying function (upvalues are runtime-specific)
        self.function.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Closure {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        let function = Function::deserialize(deserializer)?;
        Ok(Closure {
            function,
            upvalues: Vec::new(), // Upvalues must be recreated at runtime
        })
    }
}

#[derive(Debug, Clone)]
pub struct Class {
    pub name: String,
    pub methods: HashMap<String, Value>,
    pub superclass: Option<Box<Object>>,
}

impl Serialize for Class {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("Class", 3)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("methods", &self.methods)?;
        // Serialize superclass name only, not the full Object (which may contain non-serializable parts)
        if let Some(ref super_obj) = self.superclass {
            if let Object::Class(super_class) = super_obj.as_ref() {
                s.serialize_field("superclass_name", &super_class.name)?;
            } else {
                s.serialize_field("superclass_name", &"Object")?;
            }
        } else {
            s.serialize_field("superclass_name", &Option::<String>::None)?;
        }
        s.end()
    }
}

impl<'de> Deserialize<'de> for Class {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        Err(serde::de::Error::custom("Cannot deserialize Class directly - class definitions must be loaded from module"))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Upvalue {
    pub location: Option<usize>, // Stack index, None if closed
    pub closed: Option<Value>,   // Value if closed
}

impl Serialize for Upvalue {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        Err(serde::ser::Error::custom("Cannot serialize Upvalue - runtime handle"))
    }
}

impl<'de> Deserialize<'de> for Upvalue {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        Err(serde::de::Error::custom("Cannot deserialize Upvalue - runtime allocation required"))
    }
}


#[async_trait::async_trait]
pub trait HeapInterface: Send {
    fn alloc_object(&mut self, obj: Object) -> ObjHandle;
    fn get_object(&self, handle: ObjHandle) -> Option<&Object>;
    fn get_mut_object(&mut self, handle: ObjHandle) -> Option<&mut Object>;
    fn collect_garbage(&mut self);
    fn get_allocation_stats(&self) -> (usize, usize) {
        (0, 0) // Default implementation
    }
    fn set_next_gc(&mut self, size: usize);
}

impl Object {
    pub fn visit_references<F>(&self, mut f: F)
    where
        F: FnMut(ObjHandle),
    {
        match self {
             Object::Closure(c) => {
                 for &uv in &c.upvalues { f(uv); }
             },
             Object::Upvalue(uv) => {
                 if let Some(Value::Obj(h)) = &uv.closed {
                     f(*h);
                 }
             },
             Object::List(vec) => {
                 for v in vec { if let Value::Obj(h) = v { f(*h); } }
             },
             Object::Queue(vec) => {
                 for v in vec { if let Value::Obj(h) = v { f(*h); } }
             },
             Object::Map(map) | Object::Module(map) => {
                 for v in map.values() { if let Value::Obj(h) = v { f(*h); } }
             },
             Object::Class(c) => {
                 for v in c.methods.values() { if let Value::Obj(h) = v { f(*h); } }
             },
             Object::Instance(i) => {
                 // Fields
                 for v in i.fields.values() { if let Value::Obj(h) = v { f(*h); } }
                 // Class methods (implicitly reachable via class, but safer to trace class explicitly if instance keeps it alive)
                 // Instance has Arc<Class>. Class is not generic Object handle?
                 // Instance definition: pub class: Arc<Class>.
                 // Arc<Class> means Class is shared and likely NOT in the Heap via Handle?
                 // Wait, Object::Class(Class) exists.
                 // But Instance has Arc<Class>. 
                 // If Class is effectively static or managed separately?
                 // Let's check Object::Class implementation.
             },
             Object::BoundMethod(b) => {
                 if let Value::Obj(h) = b.receiver { f(h); }
             },
             Object::SharedMemory(sm) => {
                 if let Value::Obj(h) = &sm.value { f(*h); }
             },

              // Primitives and others
             Object::String(_) | Object::NativeFn(_) | Object::Function(_) | Object::Set(_) | Object::Socket(_) | Object::Listener(_) | Object::SharedBuffer(_) | Object::AtomicInt(_) | Object::Regex(_) => {},
        }
    }
}
