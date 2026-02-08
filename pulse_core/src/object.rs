use crate::value::{NativeFn, Value};
use crate::bytecode::Chunk;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjHandle(pub usize);

#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    String(String),
    NativeFn(NativeFn),
    Function(Function),
    Closure(Closure),
    List(Vec<Value>),
    Map(HashMap<String, Value>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub arity: usize,
    pub chunk: Rc<Chunk>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Closure {
    pub function: Function,
}

pub trait HeapInterface {
    fn alloc_object(&mut self, obj: Object) -> ObjHandle;
    fn get_object(&self, handle: ObjHandle) -> Option<&Object>;
    fn get_mut_object(&mut self, handle: ObjHandle) -> Option<&mut Object>;
    fn collect_garbage(&mut self);
}
