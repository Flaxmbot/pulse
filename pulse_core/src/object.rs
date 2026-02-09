use serde::{Serialize, Deserialize};
use crate::value::{NativeFn, Value};
use crate::bytecode::Chunk;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjHandle(pub usize);

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
pub struct Upvalue {
    pub location: Option<usize>, // Stack index, None if closed
    pub closed: Option<Value>,   // Value if closed
}

pub trait HeapInterface {
    fn alloc_object(&mut self, obj: Object) -> ObjHandle;
    fn get_object(&self, handle: ObjHandle) -> Option<&Object>;
    fn get_mut_object(&mut self, handle: ObjHandle) -> Option<&mut Object>;
    fn collect_garbage(&mut self);
}
