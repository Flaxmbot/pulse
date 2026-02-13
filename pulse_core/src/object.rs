use serde::{Serialize, Deserialize};
use crate::value::{NativeFn, Value};

pub use crate::value::PulseSocket;
pub use crate::value::PulseSharedMemory;
use crate::bytecode::Chunk;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjHandle(pub usize);

#[derive(Debug, Clone, PartialEq)]
pub struct SharedMemory {
    pub value: Value,
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoundMethod {
    pub receiver: Value,
    pub method: Function, // Or Closure? Usually Closure.
}

#[derive(Debug, Clone, PartialEq)]
pub struct Instance {
    pub class: Arc<Class>,
    pub fields: HashMap<String, Value>,
}

// PulseSocket moved to value.rs

#[derive(Debug, Clone, PartialEq)]
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

    SharedBuffer(PulseSharedMemory),
    Socket(PulseSocket), // Added for real networking
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

#[derive(Debug, Clone, PartialEq)]
pub struct Class {
    pub name: String,
    pub methods: HashMap<String, Value>,
    pub superclass: Option<Box<Object>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Upvalue {
    pub location: Option<usize>, // Stack index, None if closed
    pub closed: Option<Value>,   // Value if closed
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
             Object::String(_) | Object::NativeFn(_) | Object::Function(_) | Object::Set(_) | Object::Socket(_) | Object::SharedBuffer(_) => {},
        }
    }
}
