use serde::{Serialize, Deserialize};
use crate::value::{NativeFn, Value};
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

pub trait HeapInterface {
    fn alloc_object(&mut self, obj: Object) -> ObjHandle;
    fn get_object(&self, handle: ObjHandle) -> Option<&Object>;
    fn get_mut_object(&mut self, handle: ObjHandle) -> Option<&mut Object>;
    fn collect_garbage(&mut self);
    fn get_allocation_stats(&self) -> (usize, usize) {
        (0, 0) // Default implementation
    }
    fn set_next_gc(&mut self, size: usize);
}
