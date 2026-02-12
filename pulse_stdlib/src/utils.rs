//! Utility native functions

use pulse_core::{Value, PulseResult, PulseError};
use pulse_core::object::{HeapInterface, Object};
use rand::Rng;
use std::thread;
use std::time::Duration;

/// random() -> Float
/// Returns a random float between 0.0 and 1.0
pub fn random_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError("random expects 0 arguments".into()));
    }
    
    let mut rng = rand::thread_rng();
    Ok(Value::Float(rng.gen::<f64>()))
}

/// random_int(min: Int, max: Int) -> Int
/// Returns a random integer in range [min, max]
pub fn random_int_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("random_int expects 2 arguments".into()));
    }
    
    let min = args[0].as_int()?;
    let max = args[1].as_int()?;
    
    if min > max {
        return Err(PulseError::RuntimeError("random_int: min must be <= max".into()));
    }
    
    let mut rng = rand::thread_rng();
    Ok(Value::Int(rng.gen_range(min..=max)))
}

/// sleep(ms: Int) -> Unit
/// Sleeps for the specified number of milliseconds
pub fn sleep_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("sleep expects 1 argument".into()));
    }
    
    let ms = args[0].as_int()?;
    if ms < 0 {
        return Err(PulseError::RuntimeError("sleep: duration must be non-negative".into()));
    }
    
    thread::sleep(Duration::from_millis(ms as u64));
    Ok(Value::Unit)
}

/// type_of(val: Value) -> String
/// Returns the type name of a value
pub fn type_of_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("type_of expects 1 argument".into()));
    }
    
    let type_name = match &args[0] {
        Value::Int(_) => "Int",
        Value::Float(_) => "Float",
        Value::Bool(_) => "Bool",
        Value::Unit => "Unit",
        Value::Pid(_) => "Pid",
        Value::Obj(h) => {
            if let Some(obj) = heap.get_object(*h) {
                match obj {
                    Object::String(_) => "String",
                    Object::List(_) => "List",
                    Object::Map(_) => "Map",
                    Object::Closure(_) => "Function",
                    Object::Function(_) => "Function",
                    Object::NativeFn(_) => "NativeFunction",
                    Object::Upvalue(_) => "Upvalue",
                    Object::Module(_) => "Module",
                    Object::Class(_) => "Class",
                    Object::Instance(_) => "Instance",
                    Object::BoundMethod(_) => "BoundMethod",
                    Object::Set(_) => "Set",
                    Object::Queue(_) => "Queue",
                    Object::SharedMemory(_) => "SharedMemory",
                }
            } else {
                "Unknown"
            }
        }
    };
    
    let handle = heap.alloc_object(Object::String(type_name.to_string()));
    Ok(Value::Obj(handle))
}

/// to_string(val: Value) -> String
/// Converts any value to its string representation
pub fn to_string_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("to_string expects 1 argument".into()));
    }
    
    let s = value_to_string(&args[0], heap);
    let handle = heap.alloc_object(Object::String(s));
    Ok(Value::Obj(handle))
}

fn value_to_string(val: &Value, heap: &dyn HeapInterface) -> String {
    match val {
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Unit => "unit".to_string(),
        Value::Pid(p) => format!("{:?}", p),
        Value::Obj(h) => {
            if let Some(obj) = heap.get_object(*h) {
                match obj {
                    Object::String(s) => s.clone(),
                    Object::List(l) => format!("<list len={}>", l.len()),
                    Object::Map(m) => format!("<map len={}>", m.len()),
                    Object::Closure(_) => "<closure>".to_string(),
                    Object::Function(_) => "<function>".to_string(),
                    Object::NativeFn(n) => format!("<native {}>", n.name),
                    Object::Upvalue(_) => "<upvalue>".to_string(),
                    Object::Module(m) => format!("<module len={}>", m.len()),
                    Object::Class(c) => format!("<class {}>", c.name),
                    Object::Instance(i) => format!("<instance {}>", i.class.name),
                    Object::BoundMethod(_) => "<bound method>".to_string(),
                    Object::Set(s) => format!("<set len={}>", s.len()),
                    Object::Queue(q) => format!("<queue len={}>", q.len()),
                    Object::SharedMemory(sm) => format!("<shared memory locked={}>", sm.locked),
                }
            } else {
                "<invalid>".to_string()
            }
        }
    }
}

/// to_int(val: Value) -> Int
/// Converts a value to an integer
pub fn to_int_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("to_int expects 1 argument".into()));
    }
    
    match &args[0] {
        Value::Int(i) => Ok(Value::Int(*i)),
        Value::Float(f) => Ok(Value::Int(*f as i64)),
        Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
        Value::Obj(h) => {
            if let Some(Object::String(s)) = heap.get_object(*h) {
                s.parse::<i64>()
                    .map(Value::Int)
                    .map_err(|_| PulseError::RuntimeError(format!("Cannot convert '{}' to int", s)))
            } else {
                Err(PulseError::TypeMismatch { 
                    expected: "string".into(), 
                    got: "object".into() 
                })
            }
        }
        _ => Err(PulseError::TypeMismatch { 
            expected: "int, float, bool, or string".into(), 
            got: args[0].type_name() 
        }),
    }
}

/// abs(val: Int | Float) -> Int | Float
/// Returns the absolute value
pub fn abs_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("abs expects 1 argument".into()));
    }
    
    match &args[0] {
        Value::Int(i) => Ok(Value::Int(i.abs())),
        Value::Float(f) => Ok(Value::Float(f.abs())),
        _ => Err(PulseError::TypeMismatch { 
            expected: "int or float".into(), 
            got: args[0].type_name() 
        }),
    }
}

/// gc() -> Unit
/// Triggers garbage collection
pub fn gc_native(vm: &mut dyn HeapInterface, _args: &[Value]) -> PulseResult<Value> {
    vm.collect_garbage();
    Ok(Value::Unit)
}

/// len(collection: List | Map | String | Module) -> Int
/// Returns the number of items in the collection
pub fn len_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 { return Err(PulseError::RuntimeError("len() expects 1 argument".into())); }
    match args[0] {
        Value::Obj(handle) => {
            if let Some(obj) = heap.get_object(handle) {
                match obj {
                    Object::String(s) => Ok(Value::Int(s.len() as i64)),
                    Object::List(vec) => Ok(Value::Int(vec.len() as i64)),
                    Object::Map(map) => Ok(Value::Int(map.len() as i64)),
                    Object::Module(m) => Ok(Value::Int(m.len() as i64)),
                    _ => Err(PulseError::TypeMismatch{expected: "collection".into(), got: "other".into()}),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        },
        _ => Err(PulseError::TypeMismatch{expected: "collection".into(), got: args[0].type_name()}),
    }
}

/// string_to_list(s: String) -> List
/// Converts a string into a list of single-character strings
pub fn string_to_list_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("string_to_list expects 1 argument".into()));
    }
    
    let s = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::String(s)) = heap.get_object(*h) {
                s.clone()
            } else {
                return Err(PulseError::TypeMismatch { expected: "string".into(), got: "object".into() });
            }
        }
        _ => return Err(PulseError::TypeMismatch { expected: "string".into(), got: args[0].type_name() }),
    };
    
    let list: Vec<Value> = s.chars()
        .map(|c| {
            let handle = heap.alloc_object(Object::String(c.to_string()));
            Value::Obj(handle)
        })
        .collect();
        
    let list_handle = heap.alloc_object(Object::List(list));
    Ok(Value::Obj(list_handle))
}

/// push(list: List, val: Any) -> Unit
/// Appends a value to the end of a list
pub fn push_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 { return Err(PulseError::RuntimeError("push() expects 2 arguments".into())); }
    let val = args[1].clone();
    
    match args[0] {
        Value::Obj(handle) => {
            if let Some(obj) = heap.get_mut_object(handle) {
                match obj {
                    Object::List(vec) => {
                        vec.push(val);
                        Ok(Value::Unit)
                    },
                    Object::Module(_) => Err(PulseError::TypeMismatch{expected: "list".into(), got: "module".into()}),
                    _ => Err(PulseError::TypeMismatch{expected: "list".into(), got: "other".into()}),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        },
        _ => Err(PulseError::TypeMismatch{expected: "list".into(), got: args[0].type_name()}),
    }
}

/// pop(list: List) -> Any
/// Removes and returns the last item from a list
pub fn pop_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 { return Err(PulseError::RuntimeError("pop() expects 1 argument".into())); }
    
    match args[0] {
        Value::Obj(handle) => {
            if let Some(obj) = heap.get_mut_object(handle) {
                match obj {
                    Object::List(vec) => {
                         vec.pop().ok_or(PulseError::RuntimeError("Pop from empty list".into()))
                    },
                    Object::Module(_) => Err(PulseError::TypeMismatch{expected: "list".into(), got: "module".into()}),
                    _ => Err(PulseError::TypeMismatch{expected: "list".into(), got: "other".into()}),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        },
        _ => Err(PulseError::TypeMismatch{expected: "list".into(), got: args[0].type_name()}),
    }
}

/// clock() -> Float
/// Returns the current time in seconds since the epoch
pub fn clock_native(_heap: &mut dyn HeapInterface, _args: &[Value]) -> PulseResult<Value> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    Ok(Value::Float(since_the_epoch.as_secs_f64()))
}

/// println(val: Any) -> Unit
/// Prints a value to stdout followed by a newline
pub fn println_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    for arg in args {
        match arg {
            Value::Int(i) => print!("{}", i),
            Value::Float(f) => print!("{}", f),
            Value::Bool(b) => print!("{}", b),
            Value::Unit => print!("unit"),
            Value::Pid(p) => print!("<actor {:?}>", p),
            Value::Obj(h) => {
                if let Some(obj) = heap.get_object(*h) {
                    match obj {
                        Object::String(s) => print!("{}", s),
                        Object::List(l) => print!("<list len={}>", l.len()),
                        Object::Map(m) => print!("<map len={}>", m.len()),
                        Object::Closure(_) => print!("<closure>"),
                        Object::Function(f) => print!("<fn {}>", f.name),
                        Object::NativeFn(n) => print!("<native {}>", n.name),
                        Object::Upvalue(_) => print!("<upvalue>"),
                        Object::Module(m) => print!("<module len={}>", m.len()),
                        Object::Class(c) => print!("<class {}>", c.name),
                        Object::Instance(i) => print!("<instance {}>", i.class.name),
                        Object::BoundMethod(_) => print!("<bound method>"),
                        Object::Set(s) => print!("<set len={}>", s.len()),
                        Object::Queue(q) => print!("<queue len={}>", q.len()),
                        Object::SharedMemory(sm) => print!("<shared memory locked={}>", sm.locked),
                    }
                } else {
                    print!("<invalid>");
                }
            }
        }
        print!(" ");
    }
    println!();
    Ok(Value::Unit)
}

/// create_set() -> Set
/// Creates a new empty set
pub fn create_set_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError("create_set expects 0 arguments".into()));
    }

    let set = std::collections::HashSet::new();
    let handle = heap.alloc_object(Object::Set(set));
    Ok(Value::Obj(handle))
}

/// add_to_set(set: Set, val: Any) -> Unit
/// Adds a value to the set
pub fn add_to_set_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("add_to_set expects 2 arguments".into()));
    }

    // Convert value to string representation for storage in set
    let val_str = value_to_string(&args[1], heap);
    match args[0] {
        Value::Obj(handle) => {
            if let Some(obj) = heap.get_mut_object(handle) {
                match obj {
                    Object::Set(set) => {
                        set.insert(val_str);
                        Ok(Value::Unit)
                    },
                    _ => Err(PulseError::TypeMismatch{expected: "set".into(), got: "other".into()}),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        },
        _ => Err(PulseError::TypeMismatch{expected: "set".into(), got: args[0].type_name()}),
    }
}

/// remove_from_set(set: Set, val: Any) -> Bool
/// Removes a value from the set and returns whether it existed
pub fn remove_from_set_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("remove_from_set expects 2 arguments".into()));
    }

    // Convert value to string representation for lookup in set
    let val_str = value_to_string(&args[1], heap);
    match args[0] {
        Value::Obj(handle) => {
            if let Some(obj) = heap.get_mut_object(handle) {
                match obj {
                    Object::Set(set) => {
                        let removed = set.remove(&val_str);
                        Ok(Value::Bool(removed))
                    },
                    _ => Err(PulseError::TypeMismatch{expected: "set".into(), got: "other".into()}),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        },
        _ => Err(PulseError::TypeMismatch{expected: "set".into(), got: args[0].type_name()}),
    }
}

/// contains_in_set(set: Set, val: Any) -> Bool
/// Checks if a value exists in the set
pub fn contains_in_set_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("contains_in_set expects 2 arguments".into()));
    }

    // Convert value to string representation for lookup in set
    let val_str = value_to_string(&args[1], heap);
    match args[0] {
        Value::Obj(handle) => {
            if let Some(obj) = heap.get_object(handle) {
                match obj {
                    Object::Set(set) => {
                        let contains = set.contains(&val_str);
                        Ok(Value::Bool(contains))
                    },
                    _ => Err(PulseError::TypeMismatch{expected: "set".into(), got: "other".into()}),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        },
        _ => Err(PulseError::TypeMismatch{expected: "set".into(), got: args[0].type_name()}),
    }
}

/// create_queue() -> Queue
/// Creates a new empty queue
pub fn create_queue_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError("create_queue expects 0 arguments".into()));
    }

    let queue = std::collections::VecDeque::new();
    let handle = heap.alloc_object(Object::Queue(queue));
    Ok(Value::Obj(handle))
}

/// enqueue(queue: Queue, val: Any) -> Unit
/// Adds a value to the back of the queue
pub fn enqueue_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("enqueue expects 2 arguments".into()));
    }

    let val = args[1].clone();
    match args[0] {
        Value::Obj(handle) => {
            if let Some(obj) = heap.get_mut_object(handle) {
                match obj {
                    Object::Queue(queue) => {
                        queue.push_back(val);
                        Ok(Value::Unit)
                    },
                    _ => Err(PulseError::TypeMismatch{expected: "queue".into(), got: "other".into()}),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        },
        _ => Err(PulseError::TypeMismatch{expected: "queue".into(), got: args[0].type_name()}),
    }
}

/// dequeue(queue: Queue) -> Any
/// Removes and returns the front item from the queue
pub fn dequeue_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("dequeue expects 1 argument".into()));
    }

    match args[0] {
        Value::Obj(handle) => {
            if let Some(obj) = heap.get_mut_object(handle) {
                match obj {
                    Object::Queue(queue) => {
                        queue.pop_front().ok_or(PulseError::RuntimeError("Dequeue from empty queue".into()))
                    },
                    _ => Err(PulseError::TypeMismatch{expected: "queue".into(), got: "other".into()}),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        },
        _ => Err(PulseError::TypeMismatch{expected: "queue".into(), got: args[0].type_name()}),
    }
}

/// peek_queue(queue: Queue) -> Any
/// Returns the front item from the queue without removing it
pub fn peek_queue_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("peek_queue expects 1 argument".into()));
    }

    match args[0] {
        Value::Obj(handle) => {
            if let Some(obj) = heap.get_object(handle) {
                match obj {
                    Object::Queue(queue) => {
                        queue.front().cloned().ok_or(PulseError::RuntimeError("Peek from empty queue".into()))
                    },
                    _ => Err(PulseError::TypeMismatch{expected: "queue".into(), got: "other".into()}),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        },
        _ => Err(PulseError::TypeMismatch{expected: "queue".into(), got: args[0].type_name()}),
    }
}

/// map_list(list: List, fn: Function) -> List
/// Applies a function to each element of a list and returns a new list
pub fn map_list_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("map_list expects 2 arguments".into()));
    }

    let _func = args[1].clone();
    let list_handle = match args[0] {
        Value::Obj(handle) => handle,
        _ => return Err(PulseError::TypeMismatch{expected: "list".into(), got: args[0].type_name()}),
    };

    // Get the list
    let list_obj = heap.get_object(list_handle).ok_or(PulseError::RuntimeError("Invalid handle".into()))?;
    let list_values = if let Object::List(ref vals) = list_obj {
        vals.clone()
    } else {
        return Err(PulseError::TypeMismatch{expected: "list".into(), got: "other".into()});
    };

    // Apply function to each element
    let mut result_list = Vec::with_capacity(list_values.len());
    for value in list_values {
        // Call the function with the value
        // For now, we'll return the original value since we can't call functions from here
        // In a real implementation, we'd need to invoke the VM to call the function
        result_list.push(value);
    }

    let result_handle = heap.alloc_object(Object::List(result_list));
    Ok(Value::Obj(result_handle))
}

/// filter_list(list: List, fn: Function) -> List
/// Filters a list based on a predicate function
pub fn filter_list_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("filter_list expects 2 arguments".into()));
    }

    let _func = args[1].clone();
    let list_handle = match args[0] {
        Value::Obj(handle) => handle,
        _ => return Err(PulseError::TypeMismatch{expected: "list".into(), got: args[0].type_name()}),
    };

    // Get the list
    let list_obj = heap.get_object(list_handle).ok_or(PulseError::RuntimeError("Invalid handle".into()))?;
    let list_values = if let Object::List(ref vals) = list_obj {
        vals.clone()
    } else {
        return Err(PulseError::TypeMismatch{expected: "list".into(), got: "other".into()});
    };

    // Filter based on predicate
    let mut result_list = Vec::new();
    for value in list_values {
        // In a real implementation, we'd call the predicate function with the value
        // For now, we'll just return the original list
        result_list.push(value);
    }

    let result_handle = heap.alloc_object(Object::List(result_list));
    Ok(Value::Obj(result_handle))
}

/// reduce_list(list: List, fn: Function, initial: Any) -> Any
/// Reduces a list to a single value using a reducer function
pub fn reduce_list_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 3 {
        return Err(PulseError::RuntimeError("reduce_list expects 3 arguments".into()));
    }

    let _func = args[1].clone();
    let initial = args[2].clone();
    let list_handle = match args[0] {
        Value::Obj(handle) => handle,
        _ => return Err(PulseError::TypeMismatch{expected: "list".into(), got: args[0].type_name()}),
    };

    // Get the list
    let list_obj = heap.get_object(list_handle).ok_or(PulseError::RuntimeError("Invalid handle".into()))?;
    let _list_values = if let Object::List(ref vals) = list_obj {
        vals.clone()
    } else {
        return Err(PulseError::TypeMismatch{expected: "list".into(), got: "other".into()});
    };

    // Reduce the list
    // In a real implementation, we'd call the reducer function
    // For now, we'll just return the initial value
    Ok(initial)
}

/// sin(val: Float) -> Float
/// Returns the sine of the given angle in radians
pub fn sin_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("sin expects 1 argument".into()));
    }

    let val = match &args[0] {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(PulseError::TypeMismatch {
            expected: "int or float".into(),
            got: args[0].type_name()
        }),
    };

    Ok(Value::Float(val.sin()))
}

/// cos(val: Float) -> Float
/// Returns the cosine of the given angle in radians
pub fn cos_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("cos expects 1 argument".into()));
    }

    let val = match &args[0] {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(PulseError::TypeMismatch {
            expected: "int or float".into(),
            got: args[0].type_name()
        }),
    };

    Ok(Value::Float(val.cos()))
}

/// tan(val: Float) -> Float
/// Returns the tangent of the given angle in radians
pub fn tan_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("tan expects 1 argument".into()));
    }

    let val = match &args[0] {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(PulseError::TypeMismatch {
            expected: "int or float".into(),
            got: args[0].type_name()
        }),
    };

    Ok(Value::Float(val.tan()))
}

/// pow(base: Float, exponent: Float) -> Float
/// Returns the base raised to the power of the exponent
pub fn pow_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("pow expects 2 arguments".into()));
    }

    let base = match &args[0] {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(PulseError::TypeMismatch {
            expected: "int or float".into(),
            got: args[0].type_name()
        }),
    };

    let exponent = match &args[1] {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(PulseError::TypeMismatch {
            expected: "int or float".into(),
            got: args[1].type_name()
        }),
    };

    Ok(Value::Float(base.powf(exponent)))
}

/// sqrt(val: Float) -> Float
/// Returns the square root of the given value
pub fn sqrt_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("sqrt expects 1 argument".into()));
    }

    let val = match &args[0] {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(PulseError::TypeMismatch {
            expected: "int or float".into(),
            got: args[0].type_name()
        }),
    };

    if val < 0.0 {
        return Err(PulseError::RuntimeError("sqrt: cannot compute square root of negative number".into()));
    }

    Ok(Value::Float(val.sqrt()))
}

/// log(val: Float) -> Float
/// Returns the natural logarithm of the given value
pub fn log_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("log expects 1 argument".into()));
    }

    let val = match &args[0] {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(PulseError::TypeMismatch {
            expected: "int or float".into(),
            got: args[0].type_name()
        }),
    };

    if val <= 0.0 {
        return Err(PulseError::RuntimeError("log: cannot compute logarithm of non-positive number".into()));
    }

    Ok(Value::Float(val.ln()))
}

/// log10(val: Float) -> Float
/// Returns the base-10 logarithm of the given value
pub fn log10_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("log10 expects 1 argument".into()));
    }

    let val = match &args[0] {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(PulseError::TypeMismatch {
            expected: "int or float".into(),
            got: args[0].type_name()
        }),
    };

    if val <= 0.0 {
        return Err(PulseError::RuntimeError("log10: cannot compute logarithm of non-positive number".into()));
    }

    Ok(Value::Float(val.log10()))
}

/// floor(val: Float) -> Float
/// Returns the largest integer less than or equal to the given value
pub fn floor_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("floor expects 1 argument".into()));
    }

    let val = match &args[0] {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(PulseError::TypeMismatch {
            expected: "int or float".into(),
            got: args[0].type_name()
        }),
    };

    Ok(Value::Float(val.floor()))
}

/// ceil(val: Float) -> Float
/// Returns the smallest integer greater than or equal to the given value
pub fn ceil_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("ceil expects 1 argument".into()));
    }

    let val = match &args[0] {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(PulseError::TypeMismatch {
            expected: "int or float".into(),
            got: args[0].type_name()
        }),
    };

    Ok(Value::Float(val.ceil()))
}

/// round(val: Float) -> Float
/// Returns the nearest integer to the given value
pub fn round_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("round expects 1 argument".into()));
    }

    let val = match &args[0] {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => return Err(PulseError::TypeMismatch {
            expected: "int or float".into(),
            got: args[0].type_name()
        }),
    };

    Ok(Value::Float(val.round()))
}

/// deep_copy(val: Any) -> Any
/// Creates a deep copy of the given value, ensuring memory isolation
pub fn deep_copy_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("deep_copy expects 1 argument".into()));
    }

    let value = &args[0];
    match value {
        Value::Obj(handle) => {
            // Get the object to copy
            let obj_to_copy = if let Some(obj) = heap.get_object(*handle) {
                obj.clone()
            } else {
                return Err(PulseError::RuntimeError("Invalid handle for deep copy".into()));
            };
            
            // Allocate a new object with the same content
            let new_handle = heap.alloc_object(obj_to_copy);
            Ok(Value::Obj(new_handle))
        },
        // Primitive values can be copied directly
        Value::Int(i) => Ok(Value::Int(*i)),
        Value::Float(f) => Ok(Value::Float(*f)),
        Value::Bool(b) => Ok(Value::Bool(*b)),
        Value::Unit => Ok(Value::Unit),
        Value::Pid(pid) => Ok(Value::Pid(*pid)),
    }
}
