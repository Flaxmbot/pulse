use pulse_core::{Chunk, Op, Value, PulseResult, PulseError, ActorId, NativeFn, Constant};
use pulse_core::object::{Object, ObjHandle, HeapInterface, Function, Closure};

use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use crate::Heap;
#[derive(Debug, Clone)]
pub struct CallFrame {
    pub closure: ObjHandle, 
    pub ip: usize,
    pub stack_start: usize,
}

#[derive(Debug, PartialEq)]
pub enum VMStatus {
    Running,
    Yielded,
    Blocked, // Waiting for message
    Halted,
    Error(PulseError),
    // Effects
    Spawn(usize), // ip start (spawn SAME code at this offset)
    Send { target: ActorId, msg: Constant },
    Import(String), // Path to import
    Link(ActorId),      // Link to target actor
    Monitor(ActorId),   // Monitor target actor
    SpawnLink(usize),   // Spawn and link to new actor
    Register(String, ActorId),
    Unregister(String),
    WhereIs(String),
}

pub struct VM {
    // chunk: Chunk, // Removed: Chunk is now in CallFrame (via Closure)
    // ip: usize,    // Removed: IP is now in CallFrame
    pub pid: ActorId,
    pub stack: Vec<Value>,
    pub frames: Vec<CallFrame>,
    pub globals: HashMap<String, Value>,
    pub heap: Heap,
    pub open_upvalues: Vec<ObjHandle>, // Tracks upvalues still on stack
    pub loaded_modules: HashSet<String>,
}

impl VM {
    pub fn new(chunk: Chunk, pid: ActorId) -> Self {
        let mut heap = Heap::new();
        
        // Wrap script in Function/Closure
        let script_func = Function {
            arity: 0,
            chunk: Arc::new(chunk),
            name: "script".to_string(),
            upvalue_count: 0,
            module_path: None, // Default to None, Runtime sets it for main
        };
        let func_handle = heap.alloc(Object::Function(script_func.clone())); 
        // Note: Object::Function stores struct. 
        // Wait, Object::Function(Function).
        // Closure needs Function struct or handle? 
        // Closure struct: pub function: Function.
        // So we copy Function struct into Closure.
        
        let closure = Closure {
            function: script_func,
            upvalues: Vec::new(),
        };
        let closure_handle = heap.alloc(Object::Closure(closure));
        
        // Initial Frame
        let frame = CallFrame {
            closure: closure_handle,
            ip: 0,
            stack_start: 0,
        };
        
        let mut vm = Self {
            pid,
            stack: Vec::new(),
            frames: vec![frame], // Start with script frame
            globals: HashMap::new(),
            heap,
            open_upvalues: Vec::new(),
            loaded_modules: HashSet::new(),
        };
        vm.push(Value::Obj(closure_handle)); // Push script closure to slot 0

        vm.define_native("clock", clock_native);
        vm.define_native("println", println_native);
        vm.define_native("gc", gc_native);
        vm.define_native("len", len_native);
        vm.define_native("push", push_native);
        vm.define_native("pop", pop_native);
        vm
    }

    pub fn new_spawn(chunk: Arc<Chunk>, pid: ActorId, start_ip: usize) -> Self {
        let mut heap = Heap::new();
        
        let script_func = Function {
            arity: 0,
            chunk: chunk,
            name: "spawned".to_string(), 
            upvalue_count: 0,
            module_path: None,
        };
        let _ = heap.alloc(Object::Function(script_func.clone())); 
        
        let closure = Closure {
            function: script_func,
            upvalues: Vec::new(),
        };
        let closure_handle = heap.alloc(Object::Closure(closure));
        
        let frame = CallFrame {
            closure: closure_handle,
            ip: start_ip,
            stack_start: 0,
        };
        
        // Define natives... (duplicate logic? Move to helper?)
        let mut vm = Self {
            pid,
            stack: Vec::new(),
            frames: vec![frame],
            globals: HashMap::new(),
            heap,
            open_upvalues: Vec::new(),
            loaded_modules: HashSet::new(),
        };
        vm.push(Value::Obj(closure_handle));

        vm.define_native("clock", clock_native);
        vm.define_native("println", println_native);
        vm.define_native("gc", gc_native);
        vm.define_native("len", len_native);
        vm.define_native("push", push_native);
        vm.define_native("pop", pop_native);
        vm
    }

    pub fn get_current_chunk(&self) -> Arc<Chunk> {
         let frame = self.frames.last().expect("No frame");
         let closure = self.heap.get(frame.closure).expect("Closure not found");
         match closure {
             Object::Closure(c) => c.function.chunk.clone(),
             _ => panic!("Frame closure invalid"),
         }
    }
    pub fn get_current_chunk_const(&self, idx: usize) -> Constant {
        let chunk = self.get_current_chunk();
        chunk.constants[idx].clone()
    }

    pub fn define_native(&mut self, name: &str, func: fn(&mut dyn HeapInterface, &[Value]) -> PulseResult<Value>) {
        // Create NativeFn object
        let native = NativeFn { name: name.to_string(), func };
        let handle = self.heap.alloc(Object::NativeFn(native));
        
        // Put in globals
        // Verify if `define_native` also puts in native_functions list?
        // Old VM had `native_functions` map. Now we use Heap handles in Globals.
        self.globals.insert(name.to_string(), Value::Obj(handle));
    }

    pub fn run(&mut self, mut steps: usize) -> VMStatus {
        while steps > 0 {
            // Check bounds? read_byte will panic if out of bounds, or result in error?
            // Better to check.
            {
                 let frame = self.frames.last().expect("No frame");
                 let closure = self.heap.get(frame.closure).expect("Closure not found");
                 let chunk = match closure {
                     Object::Closure(c) => &c.function.chunk,
                     _ => panic!("Frame closure invalid"),
                 };
                 if frame.ip >= chunk.code.len() {
                     return VMStatus::Halted; // Or Return from script?
                 }
            }

            steps -= 1;

            let op_byte = self.read_byte();
            let op = Op::from(op_byte);
            
            match self.execute_op(op) {
                Ok(status) => {
                    if status != VMStatus::Running {
                        return status;
                    }
                }
                Err(e) => return VMStatus::Error(e),
            }
        }
        VMStatus::Running 
    }

    fn execute_op(&mut self, op: Op) -> PulseResult<VMStatus> {
        match op {
            Op::Halt => Ok(VMStatus::Halted),
            
            Op::Pop => {
                self.pop()?;
                Ok(VMStatus::Running)
            }

            Op::Dup => {
                let val = self.peek(0).clone();
                self.push(val);
                Ok(VMStatus::Running)
            }

            Op::Eq => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(Value::Bool(a == b));
                Ok(VMStatus::Running)
            }

            Op::Neq => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(Value::Bool(a != b));
                Ok(VMStatus::Running)
            }

            Op::Gt => {
                let b = self.pop()?;
                let a = self.pop()?;
                match (a, b) {
                    (Value::Int(v1), Value::Int(v2)) => self.push(Value::Bool(v1 > v2)),
                    (Value::Float(v1), Value::Float(v2)) => self.push(Value::Bool(v1 > v2)),
                    (v1, v2) => return Err(PulseError::TypeMismatch{expected: "number".into(), got: format!("{:?} vs {:?}", v1.type_name(), v2.type_name())}),
                }
                Ok(VMStatus::Running)
            }

            Op::Lt => {
                let b = self.pop()?;
                let a = self.pop()?;
                match (a, b) {
                    (Value::Int(v1), Value::Int(v2)) => self.push(Value::Bool(v1 < v2)),
                    (Value::Float(v1), Value::Float(v2)) => self.push(Value::Bool(v1 < v2)),
                    (v1, v2) => return Err(PulseError::TypeMismatch{expected: "number".into(), got: format!("{:?} vs {:?}", v1.type_name(), v2.type_name())}),
                }
                Ok(VMStatus::Running)
            }
            
            Op::Const => {
                let const_idx = self.read_byte();
                let constant = self.get_current_chunk_const(const_idx as usize);
                let val = match constant {
                    Constant::Bool(b) => Value::Bool(b),
                    Constant::Int(i) => Value::Int(i),
                    Constant::Float(f) => Value::Float(f),
                    Constant::Unit => Value::Unit,
                    Constant::String(s) => {
                        let handle = self.heap.alloc(Object::String(s));
                        Value::Obj(handle)
                    },
                    Constant::Function(func) => {
                         // Should not happen for Op::Const? Compiler emits Op::Closure?
                         // Or maybe we treat it as object?
                         // Compiler uses Op::Closure for functions.
                         // But if we have `const x = fn...`?
                         // Usually Op::Closure is used.
                         // If we encounter Function constant here, it means we are loading it raw?
                         let handle = self.heap.alloc(Object::Function(*func));
                         Value::Obj(handle)
                    }
                };
                self.push(val);
                Ok(VMStatus::Running)
            }

            Op::Add => {
                let b = self.pop()?;
                let a = self.pop()?;
                match (a, b) {
                    (Value::Int(v1), Value::Int(v2)) => self.push(Value::Int(v1 + v2)),
                    (Value::Float(v1), Value::Float(v2)) => self.push(Value::Float(v1 + v2)),
                    (Value::Obj(h1), Value::Obj(h2)) => {
                        // Check if both are strings
                        let s1 = self.get_string(h1)?;
                        let s2 = self.get_string(h2)?;
                        let new_s = s1 + &s2;
                        let handle = self.heap.alloc(Object::String(new_s));
                        self.push(Value::Obj(handle));
                    },
                    (v1, v2) => return Err(PulseError::TypeMismatch{expected: "numbers or strings".into(), got: format!("{:?} + {:?}", v1.type_name(), v2.type_name())}),
                }
                Ok(VMStatus::Running)
            }

            Op::Sub => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                self.push(Value::Int(a - b));
                Ok(VMStatus::Running)
            }

            Op::Mul => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                self.push(Value::Int(a * b));
                Ok(VMStatus::Running)
            }

            Op::Div => {
                let b = self.pop_int()?;
                let a = self.pop_int()?;
                if b == 0 {
                    return Err(PulseError::TypeMismatch { expected: "non-zero divisor".into(), got: "0".into() });
                }
                self.push(Value::Int(a / b));
                Ok(VMStatus::Running)
            }

            Op::Jump => {
                let offset = self.read_u16();
                self.frames.last_mut().unwrap().ip += offset as usize;
                Ok(VMStatus::Running)
            }

            Op::JumpIfFalse => {
                let offset = self.read_u16();
                if !self.is_truthy(self.peek(0)) {
                    self.frames.last_mut().unwrap().ip += offset as usize;
                }
                Ok(VMStatus::Running)
            }

            Op::Call => {
                let arg_count = self.read_byte() as usize;
                let callee_val = self.peek(arg_count).clone();
                
                match callee_val {
                    Value::Obj(handle) => {
                         // Check type without holding borrow too long
                         let obj_type = self.heap.get(handle).map(|o| match o {
                             Object::NativeFn(_) => 1,
                             Object::Closure(_) => 2,
                             _ => 0,
                         }).unwrap_or(0);

                         if obj_type == 1 { // Native
                             let native = if let Some(Object::NativeFn(n)) = self.heap.get(handle) { n.clone() } else { unreachable!() };
                             let args_start = self.stack.len() - arg_count;
                             let args = self.stack[args_start..].to_vec();
                             self.stack.truncate(args_start - 1); 
                             let result = (native.func)(self, &args)?;
                             self.push(result);
                             Ok(VMStatus::Running)
                         } else if obj_type == 2 { // Closure
                             let arity = if let Some(Object::Closure(c)) = self.heap.get(handle) { c.function.arity } else { unreachable!() };
                             if arg_count != arity {
                                 return Err(PulseError::RuntimeError(format!("Expected {} args, got {}", arity, arg_count)));
                             }
                             
                             let frame = CallFrame {
                                 closure: handle,
                                 ip: 0,
                                 stack_start: self.stack.len() - arg_count - 1,
                             };
                             self.frames.push(frame);
                             Ok(VMStatus::Running)
                         } else {
                             Err(PulseError::TypeMismatch{expected: "function".into(), got: "other object".into()})
                         }
                    },
                    _ => Err(PulseError::TypeMismatch{expected: "function".into(), got: callee_val.type_name()}),
                }
            }

            Op::Return => {
                let result = self.pop()?; 
                let frame = self.frames.pop().ok_or(PulseError::RuntimeError("Return from top level".into()))?;
                
                // Close upvalues for this frame's locals
                self.close_upvalues(frame.stack_start);

                self.stack.truncate(frame.stack_start);
                self.push(result);

                if self.frames.is_empty() {
                    return Ok(VMStatus::Halted);
                }
                Ok(VMStatus::Running)
            }

            Op::Closure => {
                let const_idx = self.read_byte();
                let constant = self.get_current_chunk_const(const_idx as usize);
                let function = match constant {
                    Constant::Function(f) => f,
                     _ => return Err(PulseError::TypeMismatch{expected: "function".into(), got: "other constant".into()}),
                };
                
                let mut upvalues = Vec::new();
                for _ in 0..function.upvalue_count {
                    let is_local = self.read_byte() == 1;
                    let index = self.read_byte();
                    if is_local {
                        let frame_start = self.frames.last().unwrap().stack_start;
                        upvalues.push(self.capture_upvalue(frame_start + index as usize));
                    } else {
                        // Capture from current closure's upvalues
                        let current_closure_handle = self.frames.last().unwrap().closure;
                        if let Some(Object::Closure(c)) = self.heap.get(current_closure_handle) {
                             upvalues.push(c.upvalues[index as usize]);
                        }
                    }
                }

                let closure = Closure {
                    function: *function.clone(),
                    upvalues,
                };
                let handle = self.heap.alloc(Object::Closure(closure));
                self.push(Value::Obj(handle));
                Ok(VMStatus::Running)
            }

            Op::GetUpvalue => {
                let slot = self.read_byte();
                let closure_handle = self.frames.last().unwrap().closure;
                let val = if let Some(Object::Closure(c)) = self.heap.get(closure_handle) {
                    let uv_handle = c.upvalues[slot as usize];
                    if let Some(Object::Upvalue(uv)) = self.heap.get(uv_handle) {
                        if let Some(loc) = uv.location {
                            self.stack[loc].clone()
                        } else {
                            uv.closed.as_ref().expect("Closed upvalue missing value").clone()
                        }
                    } else { return Err(PulseError::RuntimeError("Invalid upvalue".into())); }
                } else { return Err(PulseError::RuntimeError("No closure in frame".into())); };
                
                self.push(val);
                Ok(VMStatus::Running)
            }

            Op::SetUpvalue => {
                let slot = self.read_byte();
                let val = self.peek(0).clone();
                let closure_handle = self.frames.last().unwrap().closure;
                if let Some(Object::Closure(c)) = self.heap.get(closure_handle) {
                    let uv_handle = c.upvalues[slot as usize];
                    if let Some(Object::Upvalue(uv)) = self.heap.get_mut(uv_handle) {
                        if let Some(loc) = uv.location {
                            self.stack[loc] = val;
                        } else {
                            uv.closed = Some(val);
                        }
                    } else { return Err(PulseError::RuntimeError("Invalid upvalue".into())); }
                } else { return Err(PulseError::RuntimeError("No closure in frame".into())); };
                Ok(VMStatus::Running)
            }

            Op::CloseUpvalue => {
                self.close_upvalues(self.stack.len() - 1);
                self.pop()?;
                Ok(VMStatus::Running)
            }

            Op::GetLocal => {
                let slot = self.read_byte();
                let frame_start = self.frames.last().map(|f| f.stack_start).unwrap_or(0);
                let idx = frame_start + slot as usize;
                // Check bounds?
                 if idx >= self.stack.len() {
                        return Err(PulseError::StackUnderflow);
                }
                let val = self.stack[idx].clone();
                self.push(val);
                Ok(VMStatus::Running)
            }

            Op::SetLocal => {
                let slot = self.read_byte();
                let val = self.peek(0).clone(); 
                let frame_start = self.frames.last().map(|f| f.stack_start).unwrap_or(0);
                let idx = frame_start + slot as usize;
                if idx < self.stack.len() {
                    self.stack[idx] = val;
                }
                Ok(VMStatus::Running)
            }

            Op::GetGlobal => {
                let name_idx = self.read_byte();
                let constant = self.get_current_chunk_const(name_idx as usize);
                let name = match constant {
                    Constant::String(s) => s.clone(),
                    _ => return Err(PulseError::RuntimeError("Global name must be string".into())),
                };
                let val = self.globals.get(&name).ok_or(PulseError::UndefinedVariable(name.clone()))?.clone();
                self.push(val);
                Ok(VMStatus::Running)
            }

            Op::SetGlobal => {
                let name_idx = self.read_byte();
                let constant = self.get_current_chunk_const(name_idx as usize);
                let name = match constant {
                     Constant::String(s) => s.clone(),
                     _ => return Err(PulseError::RuntimeError("Global name must be string".into())),
                };
                let val = self.peek(0).clone();
                if self.globals.contains_key(&name) {
                    self.globals.insert(name, val);
                    Ok(VMStatus::Running)
                } else {
                    Err(PulseError::UndefinedVariable(name))
                }
            }

            Op::DefineGlobal => {
                let name_idx = self.read_byte();
                let constant = self.get_current_chunk_const(name_idx as usize);
                let name = match constant {
                     Constant::String(s) => s.clone(),
                     _ => return Err(PulseError::RuntimeError("Global name must be string".into())),
                };
                let val = self.pop()?;
                self.globals.insert(name, val);
                Ok(VMStatus::Running)
            }

            Op::Print => {
                let val = self.pop()?;
                self.print_value(&val);
                println!();
                Ok(VMStatus::Running)
            }

            Op::Negate => {
                let val = self.pop()?;
                match val {
                    Value::Int(n) => self.push(Value::Int(-n)),
                    Value::Float(n) => self.push(Value::Float(-n)),
                    _ => return Err(PulseError::TypeMismatch{expected: "number".into(), got: "other".into()}),
                }
                Ok(VMStatus::Running)
            }
            Op::Not => {
                let val = self.pop()?;
                let b = self.is_truthy(&val);
                self.push(Value::Bool(!b));
                Ok(VMStatus::Running)
            }

            Op::Loop => {
                let offset = self.read_u16();
                self.frames.last_mut().unwrap().ip -= offset as usize;
                Ok(VMStatus::Running)
            }


            Op::SelfId => {
                self.push(Value::Pid(self.pid));
                Ok(VMStatus::Running)
            }

            Op::Send => {
                let target_val = self.pop()?;
                let msg = self.pop()?;
                
                let target = match target_val {
                    Value::Pid(pid) => pid,
                    _ => return Err(PulseError::TypeMismatch{expected: "pid".into(), got: target_val.type_name()}),
                };
                
                // Convert msg (Value) to Constant (owned) for safe transfer
                let msg_const = match msg {
                    Value::Bool(b) => Constant::Bool(b),
                    Value::Int(i) => Constant::Int(i),
                    Value::Float(f) => Constant::Float(f),
                    Value::Unit => Constant::Unit,
                    Value::Pid(_) => return Err(PulseError::RuntimeError("Cannot send PIDs yet".into())),
                    Value::Obj(handle) => {
                        if let Some(obj) = self.heap.get(handle) {
                            match obj {
                                Object::String(s) => Constant::String(s.clone()),
                                Object::NativeFn(_) => return Err(PulseError::RuntimeError("Cannot send native functions".into())),
                                Object::List(_) | Object::Map(_) => return Err(PulseError::RuntimeError("Cannot send complex objects yet (TODO)".into())),
                                Object::Function(_) | Object::Closure(_) => return Err(PulseError::RuntimeError("Cannot send functions yet (TODO)".into())),
                                Object::Upvalue(_) => return Err(PulseError::RuntimeError("Cannot send upvalues".into())),
                            }
                        } else {
                            return Err(PulseError::RuntimeError("Cannot send freed object".into()));
                        }
                    },
                };

                Ok(VMStatus::Send { target, msg: msg_const })
            }

            Op::Receive => {
                // Return Blocked to signal Runtime to check mailbox
                Ok(VMStatus::Blocked)
            }

            Op::Spawn => {
                let offset = self.read_u16();
                Ok(VMStatus::Spawn(offset as usize))
            }

            Op::SpawnLink => {
                let offset = self.read_u16();
                Ok(VMStatus::SpawnLink(offset as usize))
            }

            Op::Link => {
                let target_val = self.pop()?;
                let target = match target_val {
                    Value::Pid(pid) => pid,
                    _ => return Err(PulseError::TypeMismatch{expected: "pid".into(), got: target_val.type_name()}),
                };
                Ok(VMStatus::Link(target))
            }

            Op::Monitor => {
                let target_val = self.pop()?;
                let target = match target_val {
                    Value::Pid(pid) => pid,
                    _ => return Err(PulseError::TypeMismatch{expected: "pid".into(), got: target_val.type_name()}),
                };
                Ok(VMStatus::Monitor(target))
            }

            Op::Register => {
                let pid_val = self.pop()?;
                let name_val = self.pop()?;
                
                let pid = match pid_val {
                    Value::Pid(p) => p,
                    _ => return Err(PulseError::TypeMismatch{expected: "pid".into(), got: pid_val.type_name()}),
                };
                
                let name = match name_val {
                    Value::Obj(h) => self.get_string(h)?,
                    _ => return Err(PulseError::TypeMismatch{expected: "string name".into(), got: name_val.type_name()}),
                };
                
                Ok(VMStatus::Register(name, pid))
            }

            Op::Unregister => {
                let name_val = self.pop()?;
                let name = match name_val {
                    Value::Obj(h) => self.get_string(h)?,
                    _ => return Err(PulseError::TypeMismatch{expected: "string name".into(), got: name_val.type_name()}),
                };
                Ok(VMStatus::Unregister(name))
            }

            Op::WhereIs => {
                let name_val = self.pop()?;
                let name = match name_val {
                    Value::Obj(h) => self.get_string(h)?,
                    _ => return Err(PulseError::TypeMismatch{expected: "string name".into(), got: name_val.type_name()}),
                };
                Ok(VMStatus::WhereIs(name))
            }

            Op::Import => {
                let path_idx = self.read_byte();
                let constant = self.get_current_chunk_const(path_idx as usize);
                let path = match constant {
                    Constant::String(s) => s.clone(),
                    _ => return Err(PulseError::TypeMismatch { expected: "string".into(), got: "other constant".into() }),
                };

                let resolved = self.resolve_path(&path)?;
                if self.loaded_modules.contains(&resolved) {
                    Ok(VMStatus::Running)
                } else {
                    Ok(VMStatus::Import(resolved))
                }
            }

            Op::BuildList => {
                let count = self.read_byte() as usize;
                let mut items = Vec::with_capacity(count);
                // Pop items in reverse order
                for _ in 0..count {
                    items.push(self.pop()?);
                }
                items.reverse(); // Restore valid order
                
                let handle = self.heap.alloc(Object::List(items));
                self.push(Value::Obj(handle));
                Ok(VMStatus::Running)
            }

            Op::BuildMap => {
                let count = self.read_byte() as usize;
                let mut map = HashMap::with_capacity(count);
                
                for _ in 0..count {
                    let val = self.pop()?;
                    let key_val = self.pop()?;
                    
                    let key = match key_val {
                        Value::Obj(h) => self.get_string(h)?,
                        _ => return Err(PulseError::TypeMismatch{expected: "string key".into(), got: key_val.type_name()}),
                    };
                    
                    map.insert(key, val);
                }
                
                let handle = self.heap.alloc(Object::Map(map));
                self.push(Value::Obj(handle));
                Ok(VMStatus::Running)
            }

            Op::GetIndex => {
                let index_val = self.pop()?;
                let target_val = self.pop()?;

                match target_val {
                    Value::Obj(handle) => {
                        let obj = self.heap.get(handle).ok_or(PulseError::RuntimeError("Invalid handle".into()))?;
                        match obj {
                            Object::List(vec) => {
                                let idx = index_val.as_int()?;
                                if idx < 0 || idx >= vec.len() as i64 {
                                    return Err(PulseError::RuntimeError(format!("Index out of bounds: {}", idx)));
                                }
                                let val = vec[idx as usize].clone();
                                self.push(val);
                            },
                            Object::Map(map) => {
                                let key = match index_val {
                                    Value::Obj(h) => self.get_string(h)?,
                                    _ => return Err(PulseError::TypeMismatch{expected: "string key".into(), got: index_val.type_name()}),
                                };
                                let val = map.get(&key).unwrap_or(&Value::Unit).clone();
                                self.push(val);
                            },
                            _ => return Err(PulseError::TypeMismatch{expected: "List or Map".into(), got: "other object".into()}),
                        }
                    },
                    _ => return Err(PulseError::TypeMismatch{expected: "List or Map".into(), got: target_val.type_name()}),
                }
                Ok(VMStatus::Running)
            }

            Op::SetIndex => {
                let val = self.pop()?; // Value to set
                let index_val = self.pop()?; // Index/Key
                let target_val = self.pop()?; // List/Map

                match target_val {
                    Value::Obj(handle) => {
                        // Pre-resolve string key if index is a string object
                        // We must do this before borrowing heap mutably for target.
                        let key_string = if let Value::Obj(h) = index_val {
                             Some(self.get_string(h)?) 
                        } else {
                             None
                        };
                        
                        let obj = self.heap.get_mut(handle).ok_or(PulseError::RuntimeError("Invalid handle".into()))?;
                        match obj {
                            Object::List(vec) => {
                                let idx = index_val.as_int()?;
                                if idx < 0 || idx >= vec.len() as i64 {
                                    return Err(PulseError::RuntimeError(format!("Index out of bounds: {}", idx)));
                                }
                                vec[idx as usize] = val.clone();
                            },
                            Object::Map(map) => {
                                let key = key_string.ok_or(PulseError::TypeMismatch{expected: "string key".into(), got: index_val.type_name()})?;
                                map.insert(key, val.clone());
                            }
                            _ => return Err(PulseError::TypeMismatch{expected: "List or Map".into(), got: "other object".into()}),
                        }
                    },
                    _ => return Err(PulseError::TypeMismatch{expected: "List or Map".into(), got: target_val.type_name()}),
                }
                
                self.push(val); // Expression result is assigned value
                Ok(VMStatus::Running)
            }

            Op::Unit => {
                self.push(Value::Unit);
                Ok(VMStatus::Running)
            }
        }
    }

    fn read_byte(&mut self) -> u8 {
        let frame = self.frames.last_mut().expect("No frame");
        let ip = frame.ip;
        let closure_handle = frame.closure;
        frame.ip += 1;
        
        let obj = self.heap.get(closure_handle).expect("Closure not found");
        let chunk = match obj {
            Object::Closure(c) => &c.function.chunk,
            _ => panic!("Frame closure is not a closure"),
        };
        chunk.code[ip]
    }

    fn read_u16(&mut self) -> u16 {
        let b1 = self.read_byte();
        let b2 = self.read_byte();
        u16::from_le_bytes([b1, b2])
    }

    fn is_truthy(&self, value: &Value) -> bool {
        match value {
            Value::Bool(b) => *b,
            Value::Unit => false,
            _ => true,
        }
    }

    fn capture_upvalue(&mut self, local_idx: usize) -> ObjHandle {
        // 1. Search for existing open upvalue
        for handle in &self.open_upvalues {
            if let Some(Object::Upvalue(uv)) = self.heap.get(*handle) {
                if uv.location == Some(local_idx) {
                    return *handle;
                }
            }
        }
        
        // 2. Create new open upvalue
        let handle = self.heap.alloc(Object::Upvalue(pulse_core::object::Upvalue {
            location: Some(local_idx),
            closed: None,
        }));
        
        // 3. Keep it in open_upvalues list
        self.open_upvalues.push(handle);
        handle
    }

    fn close_upvalues(&mut self, last_idx: usize) {
        let mut i = 0;
        while i < self.open_upvalues.len() {
            let handle = self.open_upvalues[i];
            let should_close = if let Some(Object::Upvalue(uv)) = self.heap.get(handle) {
                uv.location.map_or(false, |loc| loc >= last_idx)
            } else { false };

            if should_close {
                self.open_upvalues.remove(i);
                // Move value from stack to upvalue object
                if let Some(Object::Upvalue(uv)) = self.heap.get_mut(handle) {
                    let loc = uv.location.expect("Upvalue missing location");
                    uv.closed = Some(self.stack[loc].clone());
                    uv.location = None;
                }
            } else {
                i += 1;
            }
        }
    }

    pub fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    pub fn pop(&mut self) -> PulseResult<Value> {
        self.stack.pop().ok_or(PulseError::StackUnderflow)
    }

    fn pop_int(&mut self) -> PulseResult<i64> {
        let val = self.pop()?;
        val.as_int()
    }

    fn peek(&self, distance: usize) -> &Value {
        &self.stack[self.stack.len() - 1 - distance]
    }
    fn get_string(&self, handle: ObjHandle) -> PulseResult<String> {
        let obj = self.heap.get(handle).ok_or(PulseError::RuntimeError("Invalid handle".into()))?;
        match obj {
            Object::String(s) => Ok(s.clone()),
            _ => Err(PulseError::TypeMismatch{expected: "string".into(), got: "object".into()}),
        }
    }
    
    fn print_value(&self, val: &Value) {
        match val {
            Value::Obj(handle) => {
                if let Some(obj) = self.heap.get(*handle) {
                    match obj {
                        Object::String(s) => print!("{}", s),
                        Object::NativeFn(n) => print!("<native fn {}>", n.name),
                        Object::List(vec) => {
                            print!("[");
                            for (i, val) in vec.iter().enumerate() {
                                if i > 0 { print!(", "); }
                                self.print_value(val);
                            }
                            print!("]");
                        },
                        Object::Map(map) => {
                            print!("{{");
                            for (i, (k, v)) in map.iter().enumerate() {
                                if i > 0 { print!(", "); }
                                print!("{}: ", k);
                                self.print_value(v);
                            }
                            print!("}}");
                        },
                        Object::Function(f) => print!("<fn {}>", f.name),
                        Object::Closure(c) => print!("<fn {}>", c.function.name),
                        Object::Upvalue(_) => print!("<upvalue>"),
                    }
                } else {
                    print!("<freed object>");
                }
            },
            Value::Int(i) => print!("{}", i),
            Value::Float(f) => print!("{}", f),
            Value::Bool(b) => print!("{}", b),
            Value::Unit => print!("unit"),
            Value::Pid(pid) => print!("<actor {:?}>", pid),
        }
    }

    pub fn collect_garbage(&mut self) {
        // 1. Mark Roots
        self.mark_roots();
        
        // 2. Trace (Tri-color marking)
        self.heap.trace();
        
        // 3. Sweep
        self.heap.sweep();
    }

    fn mark_roots(&mut self) {
        // Stack
        for val in &self.stack {
            if let Value::Obj(handle) = val {
                self.heap.mark_object(*handle);
            }
        }
        
        // Globals
        for val in self.globals.values() {
            if let Value::Obj(handle) = val {
                self.heap.mark_object(*handle);
            }
        }
        
        // Call Frames (closures being executed)
        for frame in &self.frames {
            self.heap.mark_object(frame.closure);
        }

        // Open Upvalues
        for handle in &self.open_upvalues {
            self.heap.mark_object(*handle);
        }
    }

    fn resolve_path(&self, path: &str) -> PulseResult<String> {
        // Simple resolution: if relative, use current module path as base
        if path.starts_with("./") || path.starts_with("../") {
            let frame = self.frames.last().unwrap();
            let closure = self.heap.get(frame.closure).unwrap();
            if let Object::Closure(c) = closure {
                if let Some(base) = &c.function.module_path {
                   let base_path = std::path::Path::new(base);
                   if let Some(parent) = base_path.parent() {
                       let resolved = parent.join(path);
                       return Ok(resolved.to_string_lossy().to_string());
                   }
                }
            }
        }
        // Absolute or current dir
        Ok(path.to_string())
    }
}

impl HeapInterface for VM {
    fn alloc_object(&mut self, obj: Object) -> ObjHandle {
        // Trigger GC if needed?
        // Simple heuristic: if heap size > threshold
        // For now, let's just alloc.
        // In future: if self.heap.bytes_allocated > self.heap.next_gc { self.collect_garbage(); }
        self.heap.alloc(obj)
    }

    fn get_object(&self, handle: ObjHandle) -> Option<&Object> {
        self.heap.get(handle)
    }

    fn get_mut_object(&mut self, handle: ObjHandle) -> Option<&mut Object> {
        self.heap.get_mut(handle)
    }

    fn collect_garbage(&mut self) {
        self.collect_garbage();
    }
}

// Updated NativeFn Signature: fn(&mut dyn HeapInterface, &[Value]) -> PulseResult<Value>
fn clock_native(_heap: &mut dyn HeapInterface, _args: &[Value]) -> PulseResult<Value> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    Ok(Value::Float(since_the_epoch.as_secs_f64()))
}

fn println_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    for arg in args {
        // Need helper to print value given heap
        // Duplicating logic for now
        match arg {
            Value::Obj(handle) => {
                if let Some(_obj) = heap.get_object(*handle) {
                    // Start of recursion (cannot use self.print_value because we don't have VM, only HeapInterface)
                    // But HeapInterface doesn't have print helper.
                    // We need to implement a helper function that takes HeapInterface.
                    print_val_heap(heap, arg);
                } else {
                    print!("<freed object>");
                }
            },
            Value::Int(i) => print!("{}", i),
            Value::Float(f) => print!("{}", f),
            Value::Bool(b) => print!("{}", b),
            Value::Unit => print!("unit"),
            Value::Pid(pid) => print!("<actor {:?}>", pid),
        }
        print!(" ");
    }
    println!();
    Ok(Value::Unit)
}

// Helper to print value with just HeapInterface
fn print_val_heap(heap: &dyn HeapInterface, val: &Value) {
    match val {
        Value::Obj(handle) => {
             if let Some(obj) = heap.get_object(*handle) {
                 match obj {
                     Object::String(s) => print!("{}", s),
                     Object::NativeFn(n) => print!("<native fn {}>", n.name),
                     Object::List(vec) => {
                         print!("[");
                         for (i, v) in vec.iter().enumerate() {
                             if i > 0 { print!(", "); }
                             print_val_heap(heap, v);
                         }
                         print!("]");
                     },
                     Object::Map(map) => {
                         print!("{{");
                         for (i, (k, v)) in map.iter().enumerate() {
                             if i > 0 { print!(", "); }
                             print!("{}: ", k);
                             print_val_heap(heap, v);
                         }
                         print!("}}");
                     },
                     Object::Function(f) => print!("<fn {}>", f.name),
                     Object::Closure(c) => print!("<fn {}>", c.function.name),
                     Object::Upvalue(_) => print!("<upvalue>"),
                 }
             } else {
                 print!("<freed object>");
             }
        },
        Value::Int(i) => print!("{}", i),
        Value::Float(f) => print!("{}", f),
        Value::Bool(b) => print!("{}", b),
        Value::Unit => print!("unit"),
        Value::Pid(pid) => print!("<actor {:?}>", pid),
    }
}

fn gc_native(vm: &mut dyn HeapInterface, _args: &[Value]) -> PulseResult<Value> {
    vm.collect_garbage();
    Ok(Value::Unit)
}

fn len_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 { return Err(PulseError::RuntimeError("len() expects 1 argument".into())); }
    match args[0] {
        Value::Obj(handle) => {
            if let Some(obj) = heap.get_object(handle) {
                match obj {
                    Object::String(s) => Ok(Value::Int(s.len() as i64)),
                    Object::List(vec) => Ok(Value::Int(vec.len() as i64)),
                    Object::Map(map) => Ok(Value::Int(map.len() as i64)),
                    _ => Err(PulseError::TypeMismatch{expected: "collection".into(), got: "other".into()}),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        },
        _ => Err(PulseError::TypeMismatch{expected: "collection".into(), got: args[0].type_name()}),
    }
}

fn push_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    // push(list, val)
    if args.len() != 2 { return Err(PulseError::RuntimeError("push() expects 2 arguments".into())); }
    let val = args[1].clone(); // Value to push
    
    match args[0] {
        Value::Obj(handle) => {
            if let Some(obj) = heap.get_mut_object(handle) {
                match obj {
                    Object::List(vec) => {
                        vec.push(val);
                        Ok(Value::Unit)
                    },
                    _ => Err(PulseError::TypeMismatch{expected: "list".into(), got: "other".into()}),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        },
        _ => Err(PulseError::TypeMismatch{expected: "list".into(), got: args[0].type_name()}),
    }
}

fn pop_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    // pop(list) -> val
    if args.len() != 1 { return Err(PulseError::RuntimeError("pop() expects 1 argument".into())); }
    
    match args[0] {
        Value::Obj(handle) => {
            if let Some(obj) = heap.get_mut_object(handle) {
                match obj {
                    Object::List(vec) => {
                         vec.pop().ok_or(PulseError::RuntimeError("Pop from empty list".into()))
                    },
                    _ => Err(PulseError::TypeMismatch{expected: "list".into(), got: "other".into()}),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        },
        _ => Err(PulseError::TypeMismatch{expected: "list".into(), got: args[0].type_name()}),
    }
}
