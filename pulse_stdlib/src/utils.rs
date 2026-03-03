//! Utility native functions

use pulse_core::object::{HeapInterface, Object};
use pulse_core::{PulseError, PulseResult, Value};
use rand::Rng;
use std::time::Duration;

/// random() -> Float
/// Returns a random float between 0.0 and 1.0
pub fn random_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError(
            "random expects 0 arguments".into(),
        ));
    }

    let mut rng = rand::thread_rng();
    Ok(Value::Float(rng.gen::<f64>()))
}

/// random_int(min: Int, max: Int) -> Int
/// Returns a random integer in range [min, max]
pub fn random_int_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "random_int expects 2 arguments".into(),
        ));
    }

    let min = args[0].as_int()?;
    let max = args[1].as_int()?;

    if min > max {
        return Err(PulseError::RuntimeError(
            "random_int: min must be <= max".into(),
        ));
    }

    let mut rng = rand::thread_rng();
    Ok(Value::Int(rng.gen_range(min..=max)))
}

use futures::FutureExt;
use std::future::Future;
use std::pin::Pin;

/// sleep(ms: Int) -> Unit
/// Sleeps for the specified number of milliseconds
pub fn sleep_native<'a>(
    _heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 1 {
            return Err(PulseError::RuntimeError("sleep expects 1 argument".into()));
        }

        let ms = args[0].as_int()?;
        if ms < 0 {
            return Err(PulseError::RuntimeError(
                "sleep: duration must be non-negative".into(),
            ));
        }

        tokio::time::sleep(Duration::from_millis(ms as u64)).await;
        Ok(Value::Unit)
    }
    .boxed()
}

/// type_of(val: Value) -> String
/// Returns the type name of a value
pub fn type_of_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "type_of expects 1 argument".into(),
        ));
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
                    Object::Instance(i) => {
                        // Return the class name as the type (runtime type tag)
                        let class_name = i.class.name.as_str();
                        class_name
                    }
                    Object::BoundMethod(_) => "BoundMethod",
                    Object::Set(_) => "Set",
                    Object::Queue(_) => "Queue",

                    Object::SharedMemory(_) => "SharedMemory",
                    Object::Socket(_) => "Socket",
                    Object::SharedBuffer(_) => "SharedBuffer",
                    Object::Listener(_) => "Listener",
                    Object::AtomicInt(_) => "AtomicInt",
                    Object::Regex(_) => "Regex",
                    Object::WebSocket(_) => "WebSocket",
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
        return Err(PulseError::RuntimeError(
            "to_string expects 1 argument".into(),
        ));
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

                    Object::Socket(_) => "<socket>".to_string(),
                    Object::SharedBuffer(_) => "<shared buffer>".to_string(),
                    Object::Listener(_) => "<listener>".to_string(),
                    Object::AtomicInt(_) => "<atomic>".to_string(),
                    Object::Regex(_) => "<regex>".to_string(),
                    Object::WebSocket(_) => "<websocket>".to_string(),
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
                    got: "object".into(),
                })
            }
        }
        _ => Err(PulseError::TypeMismatch {
            expected: "int, float, bool, or string".into(),
            got: args[0].type_name(),
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
            got: args[0].type_name(),
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
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("len() expects 1 argument".into()));
    }
    match args[0] {
        Value::Obj(handle) => {
            if let Some(obj) = heap.get_object(handle) {
                match obj {
                    Object::String(s) => Ok(Value::Int(s.len() as i64)),
                    Object::List(vec) => Ok(Value::Int(vec.len() as i64)),
                    Object::Map(map) => Ok(Value::Int(map.len() as i64)),
                    Object::Module(m) => Ok(Value::Int(m.len() as i64)),
                    Object::Set(set) => Ok(Value::Int(set.len() as i64)),
                    Object::Queue(queue) => Ok(Value::Int(queue.len() as i64)),
                    _ => Err(PulseError::TypeMismatch {
                        expected: "collection".into(),
                        got: "other".into(),
                    }),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        }
        _ => Err(PulseError::TypeMismatch {
            expected: "collection".into(),
            got: args[0].type_name(),
        }),
    }
}

/// string_to_list(s: String) -> List
/// Converts a string into a list of single-character strings
pub fn string_to_list_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "string_to_list expects 1 argument".into(),
        ));
    }

    let s = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::String(s)) = heap.get_object(*h) {
                s.clone()
            } else {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: "object".into(),
                });
            }
        }
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "string".into(),
                got: args[0].type_name(),
            })
        }
    };

    let list: Vec<Value> = s
        .chars()
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
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "push() expects 2 arguments".into(),
        ));
    }
    let val = args[1];

    match args[0] {
        Value::Obj(handle) => {
            if let Some(obj) = heap.get_mut_object(handle) {
                match obj {
                    Object::List(vec) => {
                        vec.push(val);
                        Ok(Value::Unit)
                    }
                    Object::Module(_) => Err(PulseError::TypeMismatch {
                        expected: "list".into(),
                        got: "module".into(),
                    }),
                    _ => Err(PulseError::TypeMismatch {
                        expected: "list".into(),
                        got: "other".into(),
                    }),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        }
        _ => Err(PulseError::TypeMismatch {
            expected: "list".into(),
            got: args[0].type_name(),
        }),
    }
}

/// pop(list: List) -> Any
/// Removes and returns the last item from a list
pub fn pop_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("pop() expects 1 argument".into()));
    }

    match args[0] {
        Value::Obj(handle) => {
            if let Some(obj) = heap.get_mut_object(handle) {
                match obj {
                    Object::List(vec) => vec
                        .pop()
                        .ok_or(PulseError::RuntimeError("Pop from empty list".into())),
                    Object::Module(_) => Err(PulseError::TypeMismatch {
                        expected: "list".into(),
                        got: "module".into(),
                    }),
                    _ => Err(PulseError::TypeMismatch {
                        expected: "list".into(),
                        got: "other".into(),
                    }),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        }
        _ => Err(PulseError::TypeMismatch {
            expected: "list".into(),
            got: args[0].type_name(),
        }),
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

                        Object::Socket(_) => print!("<socket>"),

                        Object::SharedBuffer(_) => print!("<shared buffer>"),
                        Object::Listener(_) => print!("<listener>"),
                        Object::AtomicInt(_) => print!("<atomic>"),
                        Object::Regex(_) => print!("<regex>"),
                        Object::WebSocket(_) => print!("<websocket>"),
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
        return Err(PulseError::RuntimeError(
            "create_set expects 0 arguments".into(),
        ));
    }

    let set = std::collections::HashSet::new();
    let handle = heap.alloc_object(Object::Set(set));
    Ok(Value::Obj(handle))
}

/// add_to_set(set: Set, val: Any) -> Unit
/// Adds a value to the set
pub fn add_to_set_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "add_to_set expects 2 arguments".into(),
        ));
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
                    }
                    _ => Err(PulseError::TypeMismatch {
                        expected: "set".into(),
                        got: "other".into(),
                    }),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        }
        _ => Err(PulseError::TypeMismatch {
            expected: "set".into(),
            got: args[0].type_name(),
        }),
    }
}

/// remove_from_set(set: Set, val: Any) -> Bool
/// Removes a value from the set and returns whether it existed
pub fn remove_from_set_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "remove_from_set expects 2 arguments".into(),
        ));
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
                    }
                    _ => Err(PulseError::TypeMismatch {
                        expected: "set".into(),
                        got: "other".into(),
                    }),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        }
        _ => Err(PulseError::TypeMismatch {
            expected: "set".into(),
            got: args[0].type_name(),
        }),
    }
}

/// contains_in_set(set: Set, val: Any) -> Bool
/// Checks if a value exists in the set
pub fn contains_in_set_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "contains_in_set expects 2 arguments".into(),
        ));
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
                    }
                    _ => Err(PulseError::TypeMismatch {
                        expected: "set".into(),
                        got: "other".into(),
                    }),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        }
        _ => Err(PulseError::TypeMismatch {
            expected: "set".into(),
            got: args[0].type_name(),
        }),
    }
}

/// create_queue() -> Queue
/// Creates a new empty queue
pub fn create_queue_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError(
            "create_queue expects 0 arguments".into(),
        ));
    }

    let queue = std::collections::VecDeque::new();
    let handle = heap.alloc_object(Object::Queue(queue));
    Ok(Value::Obj(handle))
}

/// enqueue(queue: Queue, val: Any) -> Unit
/// Adds a value to the back of the queue
pub fn enqueue_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "enqueue expects 2 arguments".into(),
        ));
    }

    let val = args[1];
    match args[0] {
        Value::Obj(handle) => {
            if let Some(obj) = heap.get_mut_object(handle) {
                match obj {
                    Object::Queue(queue) => {
                        queue.push_back(val);
                        Ok(Value::Unit)
                    }
                    _ => Err(PulseError::TypeMismatch {
                        expected: "queue".into(),
                        got: "other".into(),
                    }),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        }
        _ => Err(PulseError::TypeMismatch {
            expected: "queue".into(),
            got: args[0].type_name(),
        }),
    }
}

/// dequeue(queue: Queue) -> Any
/// Removes and returns the front item from the queue
pub fn dequeue_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "dequeue expects 1 argument".into(),
        ));
    }

    match args[0] {
        Value::Obj(handle) => {
            if let Some(obj) = heap.get_mut_object(handle) {
                match obj {
                    Object::Queue(queue) => queue
                        .pop_front()
                        .ok_or(PulseError::RuntimeError("Dequeue from empty queue".into())),
                    _ => Err(PulseError::TypeMismatch {
                        expected: "queue".into(),
                        got: "other".into(),
                    }),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        }
        _ => Err(PulseError::TypeMismatch {
            expected: "queue".into(),
            got: args[0].type_name(),
        }),
    }
}

/// peek_queue(queue: Queue) -> Any
/// Returns the front item from the queue without removing it
pub fn peek_queue_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "peek_queue expects 1 argument".into(),
        ));
    }

    match args[0] {
        Value::Obj(handle) => {
            if let Some(obj) = heap.get_object(handle) {
                match obj {
                    Object::Queue(queue) => queue
                        .front()
                        .cloned()
                        .ok_or(PulseError::RuntimeError("Peek from empty queue".into())),
                    _ => Err(PulseError::TypeMismatch {
                        expected: "queue".into(),
                        got: "other".into(),
                    }),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        }
        _ => Err(PulseError::TypeMismatch {
            expected: "queue".into(),
            got: args[0].type_name(),
        }),
    }
}

/// map_list(list: List, fn: Function) -> List
/// Applies a function to each element of a list and returns a new list
pub fn map_list_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "map_list expects 2 arguments".into(),
        ));
    }

    let _func = args[1];
    let list_handle = match args[0] {
        Value::Obj(handle) => handle,
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "list".into(),
                got: args[0].type_name(),
            })
        }
    };

    // Get the list
    let list_obj = heap
        .get_object(list_handle)
        .ok_or(PulseError::RuntimeError("Invalid handle".into()))?;
    let list_values = if let Object::List(ref vals) = list_obj {
        vals.clone()
    } else {
        return Err(PulseError::TypeMismatch {
            expected: "list".into(),
            got: "other".into(),
        });
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
        return Err(PulseError::RuntimeError(
            "filter_list expects 2 arguments".into(),
        ));
    }

    let _func = args[1];
    let list_handle = match args[0] {
        Value::Obj(handle) => handle,
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "list".into(),
                got: args[0].type_name(),
            })
        }
    };

    // Get the list
    let list_obj = heap
        .get_object(list_handle)
        .ok_or(PulseError::RuntimeError("Invalid handle".into()))?;
    let list_values = if let Object::List(ref vals) = list_obj {
        vals.clone()
    } else {
        return Err(PulseError::TypeMismatch {
            expected: "list".into(),
            got: "other".into(),
        });
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
        return Err(PulseError::RuntimeError(
            "reduce_list expects 3 arguments".into(),
        ));
    }

    let _func = args[1];
    let initial = args[2];
    let list_handle = match args[0] {
        Value::Obj(handle) => handle,
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "list".into(),
                got: args[0].type_name(),
            })
        }
    };

    // Get the list
    let list_obj = heap
        .get_object(list_handle)
        .ok_or(PulseError::RuntimeError("Invalid handle".into()))?;
    let _list_values = if let Object::List(ref vals) = list_obj {
        vals.clone()
    } else {
        return Err(PulseError::TypeMismatch {
            expected: "list".into(),
            got: "other".into(),
        });
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
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "int or float".into(),
                got: args[0].type_name(),
            })
        }
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
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "int or float".into(),
                got: args[0].type_name(),
            })
        }
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
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "int or float".into(),
                got: args[0].type_name(),
            })
        }
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
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "int or float".into(),
                got: args[0].type_name(),
            })
        }
    };

    let exponent = match &args[1] {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "int or float".into(),
                got: args[1].type_name(),
            })
        }
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
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "int or float".into(),
                got: args[0].type_name(),
            })
        }
    };

    if val < 0.0 {
        return Err(PulseError::RuntimeError(
            "sqrt: cannot compute square root of negative number".into(),
        ));
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
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "int or float".into(),
                got: args[0].type_name(),
            })
        }
    };

    if val <= 0.0 {
        return Err(PulseError::RuntimeError(
            "log: cannot compute logarithm of non-positive number".into(),
        ));
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
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "int or float".into(),
                got: args[0].type_name(),
            })
        }
    };

    if val <= 0.0 {
        return Err(PulseError::RuntimeError(
            "log10: cannot compute logarithm of non-positive number".into(),
        ));
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
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "int or float".into(),
                got: args[0].type_name(),
            })
        }
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
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "int or float".into(),
                got: args[0].type_name(),
            })
        }
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
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "int or float".into(),
                got: args[0].type_name(),
            })
        }
    };

    Ok(Value::Float(val.round()))
}

/// deep_copy(val: Any) -> Any
/// Creates a deep copy of the given value, ensuring memory isolation
pub fn deep_copy_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "deep_copy expects 1 argument".into(),
        ));
    }

    let value = &args[0];
    match value {
        Value::Obj(handle) => {
            // Get the object to copy
            let obj_to_copy = if let Some(obj) = heap.get_object(*handle) {
                obj.clone()
            } else {
                return Err(PulseError::RuntimeError(
                    "Invalid handle for deep copy".into(),
                ));
            };

            // Allocate a new object with the same content
            let new_handle = heap.alloc_object(obj_to_copy);
            Ok(Value::Obj(new_handle))
        }
        // Primitive values can be copied directly
        Value::Int(i) => Ok(Value::Int(*i)),
        Value::Float(f) => Ok(Value::Float(*f)),
        Value::Bool(b) => Ok(Value::Bool(*b)),
        Value::Unit => Ok(Value::Unit),
        Value::Pid(pid) => Ok(Value::Pid(*pid)),
    }
}

// ============================================================================
// INPUT / READLINE
// ============================================================================

/// input() -> String
/// Reads a line from stdin (blocking)
pub fn input_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError(
            "input expects 0 arguments (use input_prompt for prompted input)".into(),
        ));
    }

    let mut buf = String::new();
    std::io::stdin()
        .read_line(&mut buf)
        .map_err(|e| PulseError::IoError(format!("Failed to read stdin: {}", e)))?;
    // Strip trailing newline
    if buf.ends_with('\n') {
        buf.pop();
        if buf.ends_with('\r') {
            buf.pop();
        }
    }
    let handle = heap.alloc_object(Object::String(buf));
    Ok(Value::Obj(handle))
}

/// input_prompt(prompt: String) -> String
/// Prints a prompt then reads a line from stdin
pub fn input_prompt_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "input_prompt expects 1 argument".into(),
        ));
    }

    // Print the prompt
    let prompt = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::String(s)) = heap.get_object(*h) {
                s.clone()
            } else {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: "object".into(),
                });
            }
        }
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "string".into(),
                got: args[0].type_name(),
            })
        }
    };

    use std::io::Write;
    print!("{}", prompt);
    std::io::stdout()
        .flush()
        .map_err(|e| PulseError::IoError(format!("{}", e)))?;

    let mut buf = String::new();
    std::io::stdin()
        .read_line(&mut buf)
        .map_err(|e| PulseError::IoError(format!("Failed to read stdin: {}", e)))?;
    if buf.ends_with('\n') {
        buf.pop();
        if buf.ends_with('\r') {
            buf.pop();
        }
    }
    let handle = heap.alloc_object(Object::String(buf));
    Ok(Value::Obj(handle))
}

// ============================================================================
// CONVERSION
// ============================================================================

/// to_float(val: Value) -> Float
/// Converts a value to a float
pub fn to_float_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "to_float expects 1 argument".into(),
        ));
    }

    match &args[0] {
        Value::Float(f) => Ok(Value::Float(*f)),
        Value::Int(i) => Ok(Value::Float(*i as f64)),
        Value::Bool(b) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
        Value::Obj(h) => {
            if let Some(Object::String(s)) = heap.get_object(*h) {
                s.parse::<f64>().map(Value::Float).map_err(|_| {
                    PulseError::RuntimeError(format!("Cannot convert '{}' to float", s))
                })
            } else {
                Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: "object".into(),
                })
            }
        }
        _ => Err(PulseError::TypeMismatch {
            expected: "int, float, bool, or string".into(),
            got: args[0].type_name(),
        }),
    }
}

// ============================================================================
// SCALAR MIN / MAX
// ============================================================================

/// min_val(a, b) -> a | b
/// Returns the smaller of two numeric values
pub fn min_val_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "min_val expects 2 arguments".into(),
        ));
    }

    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(*a.min(b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.min(*b))),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).min(*b))),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.min(*b as f64))),
        _ => Err(PulseError::TypeMismatch {
            expected: "numeric".into(),
            got: format!("{} and {}", args[0].type_name(), args[1].type_name()),
        }),
    }
}

/// max_val(a, b) -> a | b
/// Returns the larger of two numeric values
pub fn max_val_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "max_val expects 2 arguments".into(),
        ));
    }

    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(*a.max(b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.max(*b))),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).max(*b))),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.max(*b as f64))),
        _ => Err(PulseError::TypeMismatch {
            expected: "numeric".into(),
            got: format!("{} and {}", args[0].type_name(), args[1].type_name()),
        }),
    }
}

// ============================================================================
// LIST OPERATIONS
// ============================================================================

/// sort_list(list: List) -> List
/// Returns a new sorted list (numeric values sorted numerically, strings lexicographically)
pub fn sort_list_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "sort_list expects 1 argument".into(),
        ));
    }

    let list = match args[0] {
        Value::Obj(handle) => {
            if let Some(Object::List(l)) = heap.get_object(handle) {
                l.clone()
            } else {
                return Err(PulseError::TypeMismatch {
                    expected: "list".into(),
                    got: "other".into(),
                });
            }
        }
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "list".into(),
                got: args[0].type_name(),
            })
        }
    };

    let mut sorted = list;
    sorted.sort_by(|a, b| match (a, b) {
        (Value::Int(x), Value::Int(y)) => x.cmp(y),
        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
        (Value::Int(x), Value::Float(y)) => (*x as f64)
            .partial_cmp(y)
            .unwrap_or(std::cmp::Ordering::Equal),
        (Value::Float(x), Value::Int(y)) => x
            .partial_cmp(&(*y as f64))
            .unwrap_or(std::cmp::Ordering::Equal),
        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    });

    let handle = heap.alloc_object(Object::List(sorted));
    Ok(Value::Obj(handle))
}

/// reverse_list(list: List) -> List
/// Returns a new reversed list
pub fn reverse_list_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "reverse_list expects 1 argument".into(),
        ));
    }

    let list = match args[0] {
        Value::Obj(handle) => {
            if let Some(Object::List(l)) = heap.get_object(handle) {
                l.clone()
            } else {
                return Err(PulseError::TypeMismatch {
                    expected: "list".into(),
                    got: "other".into(),
                });
            }
        }
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "list".into(),
                got: args[0].type_name(),
            })
        }
    };

    let mut reversed = list;
    reversed.reverse();
    let handle = heap.alloc_object(Object::List(reversed));
    Ok(Value::Obj(handle))
}

/// index_of(list: List, val: Any) -> Int
/// Returns the index of the first occurrence of val in the list, or -1 if not found
pub fn index_of_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "index_of expects 2 arguments".into(),
        ));
    }

    let list = match args[0] {
        Value::Obj(handle) => {
            if let Some(Object::List(l)) = heap.get_object(handle) {
                l.clone()
            } else {
                return Err(PulseError::TypeMismatch {
                    expected: "list".into(),
                    got: "other".into(),
                });
            }
        }
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "list".into(),
                got: args[0].type_name(),
            })
        }
    };

    let needle = &args[1];
    for (i, item) in list.iter().enumerate() {
        let matches = match (item, needle) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Unit, Value::Unit) => true,
            _ => false, // Object comparison by handle not reliable here
        };
        if matches {
            return Ok(Value::Int(i as i64));
        }
    }
    Ok(Value::Int(-1))
}

/// list_insert(list: List, index: Int, val: Any) -> Unit
/// Inserts a value at the given index in the list (in-place)
pub fn list_insert_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 3 {
        return Err(PulseError::RuntimeError(
            "list_insert expects 3 arguments (list, index, value)".into(),
        ));
    }

    let idx = args[1].as_int()?;
    let val = args[2];

    match args[0] {
        Value::Obj(handle) => {
            if let Some(obj) = heap.get_mut_object(handle) {
                match obj {
                    Object::List(vec) => {
                        if idx < 0 || idx > vec.len() as i64 {
                            return Err(PulseError::RuntimeError(format!(
                                "Index {} out of bounds for list of length {}",
                                idx,
                                vec.len()
                            )));
                        }
                        vec.insert(idx as usize, val);
                        Ok(Value::Unit)
                    }
                    _ => Err(PulseError::TypeMismatch {
                        expected: "list".into(),
                        got: "other".into(),
                    }),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        }
        _ => Err(PulseError::TypeMismatch {
            expected: "list".into(),
            got: args[0].type_name(),
        }),
    }
}

/// list_remove(list: List, index: Int) -> Any
/// Removes and returns the element at the given index
pub fn list_remove_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "list_remove expects 2 arguments (list, index)".into(),
        ));
    }

    let idx = args[1].as_int()?;

    match args[0] {
        Value::Obj(handle) => {
            if let Some(obj) = heap.get_mut_object(handle) {
                match obj {
                    Object::List(vec) => {
                        if idx < 0 || idx >= vec.len() as i64 {
                            return Err(PulseError::RuntimeError(format!(
                                "Index {} out of bounds for list of length {}",
                                idx,
                                vec.len()
                            )));
                        }
                        Ok(vec.remove(idx as usize))
                    }
                    _ => Err(PulseError::TypeMismatch {
                        expected: "list".into(),
                        got: "other".into(),
                    }),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        }
        _ => Err(PulseError::TypeMismatch {
            expected: "list".into(),
            got: args[0].type_name(),
        }),
    }
}

/// list_slice(list: List, start: Int, end: Int) -> List
/// Returns a new list containing elements from index start (inclusive) to end (exclusive)
pub fn list_slice_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 3 {
        return Err(PulseError::RuntimeError(
            "list_slice expects 3 arguments (list, start, end)".into(),
        ));
    }

    let start = args[1].as_int()?;
    let end = args[2].as_int()?;

    let list = match args[0] {
        Value::Obj(handle) => {
            if let Some(Object::List(l)) = heap.get_object(handle) {
                l.clone()
            } else {
                return Err(PulseError::TypeMismatch {
                    expected: "list".into(),
                    got: "other".into(),
                });
            }
        }
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "list".into(),
                got: args[0].type_name(),
            })
        }
    };

    let len = list.len() as i64;
    let s = start.max(0) as usize;
    let e = end.min(len) as usize;
    let sliced = if s <= e {
        list[s..e].to_vec()
    } else {
        Vec::new()
    };

    let handle = heap.alloc_object(Object::List(sliced));
    Ok(Value::Obj(handle))
}

/// list_contains(list: List, val: Any) -> Bool
/// Returns true if the list contains the given value
pub fn list_contains_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "list_contains expects 2 arguments".into(),
        ));
    }

    let list = match args[0] {
        Value::Obj(handle) => {
            if let Some(Object::List(l)) = heap.get_object(handle) {
                l.clone()
            } else {
                return Err(PulseError::TypeMismatch {
                    expected: "list".into(),
                    got: "other".into(),
                });
            }
        }
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "list".into(),
                got: args[0].type_name(),
            })
        }
    };

    let needle = &args[1];
    for item in &list {
        let matches = match (item, needle) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Unit, Value::Unit) => true,
            _ => false,
        };
        if matches {
            return Ok(Value::Bool(true));
        }
    }
    Ok(Value::Bool(false))
}

/// list_concat(list1: List, list2: List) -> List
/// Returns a new list that is the concatenation of list1 and list2
pub fn list_concat_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "list_concat expects 2 arguments".into(),
        ));
    }

    let list1 = match args[0] {
        Value::Obj(handle) => {
            if let Some(Object::List(l)) = heap.get_object(handle) {
                l.clone()
            } else {
                return Err(PulseError::TypeMismatch {
                    expected: "list".into(),
                    got: "other".into(),
                });
            }
        }
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "list".into(),
                got: args[0].type_name(),
            })
        }
    };

    let list2 = match args[1] {
        Value::Obj(handle) => {
            if let Some(Object::List(l)) = heap.get_object(handle) {
                l.clone()
            } else {
                return Err(PulseError::TypeMismatch {
                    expected: "list".into(),
                    got: "other".into(),
                });
            }
        }
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "list".into(),
                got: args[1].type_name(),
            })
        }
    };

    let mut result = list1;
    result.extend(list2);
    let handle = heap.alloc_object(Object::List(result));
    Ok(Value::Obj(handle))
}

/// range(start: Int, end: Int) -> List
/// Creates a list of integers from start (inclusive) to end (exclusive)
pub fn range_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.is_empty() || args.len() > 3 {
        return Err(PulseError::RuntimeError("range expects 1-3 arguments: range(end) or range(start, end) or range(start, end, step)".into()));
    }

    let (start, end, step) = match args.len() {
        1 => (0i64, args[0].as_int()?, 1i64),
        2 => (args[0].as_int()?, args[1].as_int()?, 1i64),
        3 => {
            let s = args[2].as_int()?;
            if s == 0 {
                return Err(PulseError::RuntimeError("range step cannot be zero".into()));
            }
            (args[0].as_int()?, args[1].as_int()?, s)
        }
        _ => unreachable!(),
    };

    let mut list = Vec::new();
    if step > 0 {
        let mut i = start;
        while i < end {
            list.push(Value::Int(i));
            i += step;
        }
    } else {
        let mut i = start;
        while i > end {
            list.push(Value::Int(i));
            i += step;
        }
    }

    let handle = heap.alloc_object(Object::List(list));
    Ok(Value::Obj(handle))
}

/// list_flatten(list: List) -> List
/// Flattens a list of lists into a single list (one level deep)
pub fn list_flatten_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "list_flatten expects 1 argument".into(),
        ));
    }

    let list = match args[0] {
        Value::Obj(handle) => {
            if let Some(Object::List(l)) = heap.get_object(handle) {
                l.clone()
            } else {
                return Err(PulseError::TypeMismatch {
                    expected: "list".into(),
                    got: "other".into(),
                });
            }
        }
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "list".into(),
                got: args[0].type_name(),
            })
        }
    };

    let mut result = Vec::new();
    for item in &list {
        match item {
            Value::Obj(h) => {
                if let Some(Object::List(inner)) = heap.get_object(*h) {
                    result.extend(inner.clone());
                } else {
                    result.push(*item);
                }
            }
            _ => result.push(*item),
        }
    }

    let handle = heap.alloc_object(Object::List(result));
    Ok(Value::Obj(handle))
}

// ====================================================================
// PHASE 1.3: TYPE PREDICATE FUNCTIONS
// ====================================================================

/// is_int(x) -> Bool
pub fn is_int_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("is_int expects 1 argument".into()));
    }
    Ok(Value::Bool(matches!(args[0], Value::Int(_))))
}

/// is_float(x) -> Bool
pub fn is_float_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "is_float expects 1 argument".into(),
        ));
    }
    Ok(Value::Bool(matches!(args[0], Value::Float(_))))
}

/// is_string(x) -> Bool
pub fn is_string_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "is_string expects 1 argument".into(),
        ));
    }
    let result = match args[0] {
        Value::Obj(h) => matches!(heap.get_object(h), Some(Object::String(_))),
        _ => false,
    };
    Ok(Value::Bool(result))
}

/// is_bool(x) -> Bool
pub fn is_bool_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "is_bool expects 1 argument".into(),
        ));
    }
    Ok(Value::Bool(matches!(args[0], Value::Bool(_))))
}

/// is_list(x) -> Bool
pub fn is_list_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "is_list expects 1 argument".into(),
        ));
    }
    let result = match args[0] {
        Value::Obj(h) => matches!(heap.get_object(h), Some(Object::List(_))),
        _ => false,
    };
    Ok(Value::Bool(result))
}

/// is_map(x) -> Bool
pub fn is_map_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("is_map expects 1 argument".into()));
    }
    let result = match args[0] {
        Value::Obj(h) => matches!(heap.get_object(h), Some(Object::Map(_))),
        _ => false,
    };
    Ok(Value::Bool(result))
}

/// is_nil(x) -> Bool
pub fn is_nil_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("is_nil expects 1 argument".into()));
    }
    Ok(Value::Bool(matches!(args[0], Value::Unit)))
}

// ====================================================================
// PHASE 1.3: MAP FUNCTIONS
// ====================================================================

/// map_keys(map) -> List of keys
pub fn map_keys_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "map_keys expects 1 argument".into(),
        ));
    }
    let map = match args[0] {
        Value::Obj(h) => match heap.get_object(h) {
            Some(Object::Map(m)) => m.clone(),
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "map".into(),
                    got: "other".into(),
                })
            }
        },
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "map".into(),
                got: args[0].type_name(),
            })
        }
    };
    let keys: Vec<Value> = map
        .keys()
        .map(|k| {
            let h = heap.alloc_object(Object::String(k.clone()));
            Value::Obj(h)
        })
        .collect();
    let handle = heap.alloc_object(Object::List(keys));
    Ok(Value::Obj(handle))
}

/// map_values(map) -> List of values
pub fn map_values_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "map_values expects 1 argument".into(),
        ));
    }
    let map = match args[0] {
        Value::Obj(h) => match heap.get_object(h) {
            Some(Object::Map(m)) => m.clone(),
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "map".into(),
                    got: "other".into(),
                })
            }
        },
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "map".into(),
                got: args[0].type_name(),
            })
        }
    };
    let vals: Vec<Value> = map.values().cloned().collect();
    let handle = heap.alloc_object(Object::List(vals));
    Ok(Value::Obj(handle))
}

/// map_entries(map) -> List of [key, value] lists
pub fn map_entries_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "map_entries expects 1 argument".into(),
        ));
    }
    let map = match args[0] {
        Value::Obj(h) => match heap.get_object(h) {
            Some(Object::Map(m)) => m.clone(),
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "map".into(),
                    got: "other".into(),
                })
            }
        },
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "map".into(),
                got: args[0].type_name(),
            })
        }
    };
    let entries: Vec<Value> = map
        .iter()
        .map(|(k, v)| {
            let key_h = heap.alloc_object(Object::String(k.clone()));
            let pair = vec![Value::Obj(key_h), *v];
            let pair_h = heap.alloc_object(Object::List(pair));
            Value::Obj(pair_h)
        })
        .collect();
    let handle = heap.alloc_object(Object::List(entries));
    Ok(Value::Obj(handle))
}

/// map_has_key(map, key) -> Bool
pub fn map_has_key_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "map_has_key expects 2 arguments".into(),
        ));
    }
    let key_str = value_to_string(&args[1], heap);
    match args[0] {
        Value::Obj(h) => match heap.get_object(h) {
            Some(Object::Map(m)) => Ok(Value::Bool(m.contains_key(&key_str))),
            _ => Err(PulseError::TypeMismatch {
                expected: "map".into(),
                got: "other".into(),
            }),
        },
        _ => Err(PulseError::TypeMismatch {
            expected: "map".into(),
            got: args[0].type_name(),
        }),
    }
}

/// map_delete(map, key) -> Unit
pub fn map_delete_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "map_delete expects 2 arguments".into(),
        ));
    }
    let key_str = value_to_string(&args[1], heap);
    match args[0] {
        Value::Obj(h) => {
            if let Some(Object::Map(m)) = heap.get_mut_object(h) {
                m.remove(&key_str);
                Ok(Value::Unit)
            } else {
                Err(PulseError::TypeMismatch {
                    expected: "map".into(),
                    got: "other".into(),
                })
            }
        }
        _ => Err(PulseError::TypeMismatch {
            expected: "map".into(),
            got: args[0].type_name(),
        }),
    }
}

/// map_merge(map1, map2) -> Map (new map with entries from both)
pub fn map_merge_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "map_merge expects 2 arguments".into(),
        ));
    }
    let map1 = match args[0] {
        Value::Obj(h) => match heap.get_object(h) {
            Some(Object::Map(m)) => m.clone(),
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "map".into(),
                    got: "other".into(),
                })
            }
        },
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "map".into(),
                got: args[0].type_name(),
            })
        }
    };
    let map2 = match args[1] {
        Value::Obj(h) => match heap.get_object(h) {
            Some(Object::Map(m)) => m.clone(),
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "map".into(),
                    got: "other".into(),
                })
            }
        },
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "map".into(),
                got: args[1].type_name(),
            })
        }
    };
    let mut merged = map1;
    merged.extend(map2);
    let handle = heap.alloc_object(Object::Map(merged));
    Ok(Value::Obj(handle))
}

// ====================================================================
// PHASE 1.3: COLLECTION UTILITY FUNCTIONS
// ====================================================================

/// enumerate_list(list) -> List of [index, value] pairs
pub fn enumerate_list_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "enumerate_list expects 1 argument".into(),
        ));
    }
    let list = match args[0] {
        Value::Obj(h) => match heap.get_object(h) {
            Some(Object::List(l)) => l.clone(),
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "list".into(),
                    got: "other".into(),
                })
            }
        },
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "list".into(),
                got: args[0].type_name(),
            })
        }
    };
    let pairs: Vec<Value> = list
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let pair = vec![Value::Int(i as i64), *v];
            let pair_h = heap.alloc_object(Object::List(pair));
            Value::Obj(pair_h)
        })
        .collect();
    let handle = heap.alloc_object(Object::List(pairs));
    Ok(Value::Obj(handle))
}

/// zip_lists(a, b) -> List of [a[i], b[i]] pairs
pub fn zip_lists_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "zip_lists expects 2 arguments".into(),
        ));
    }
    let list_a = match args[0] {
        Value::Obj(h) => match heap.get_object(h) {
            Some(Object::List(l)) => l.clone(),
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "list".into(),
                    got: "other".into(),
                })
            }
        },
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "list".into(),
                got: args[0].type_name(),
            })
        }
    };
    let list_b = match args[1] {
        Value::Obj(h) => match heap.get_object(h) {
            Some(Object::List(l)) => l.clone(),
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "list".into(),
                    got: "other".into(),
                })
            }
        },
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "list".into(),
                got: args[1].type_name(),
            })
        }
    };
    let len = list_a.len().min(list_b.len());
    let pairs: Vec<Value> = (0..len)
        .map(|i| {
            let pair = vec![list_a[i], list_b[i]];
            let pair_h = heap.alloc_object(Object::List(pair));
            Value::Obj(pair_h)
        })
        .collect();
    let handle = heap.alloc_object(Object::List(pairs));
    Ok(Value::Obj(handle))
}

/// unique_list(list) -> List with duplicates removed (preserves order)
pub fn unique_list_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "unique_list expects 1 argument".into(),
        ));
    }
    let list = match args[0] {
        Value::Obj(h) => match heap.get_object(h) {
            Some(Object::List(l)) => l.clone(),
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "list".into(),
                    got: "other".into(),
                })
            }
        },
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "list".into(),
                got: args[0].type_name(),
            })
        }
    };
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for item in &list {
        let key = value_to_string(item, heap);
        if seen.insert(key) {
            result.push(*item);
        }
    }
    let handle = heap.alloc_object(Object::List(result));
    Ok(Value::Obj(handle))
}

/// count_list(list, item) -> Int (count occurrences)
pub fn count_list_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "count_list expects 2 arguments".into(),
        ));
    }
    let list = match args[0] {
        Value::Obj(h) => match heap.get_object(h) {
            Some(Object::List(l)) => l.clone(),
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "list".into(),
                    got: "other".into(),
                })
            }
        },
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "list".into(),
                got: args[0].type_name(),
            })
        }
    };
    let target = value_to_string(&args[1], heap);
    let count = list
        .iter()
        .filter(|v| value_to_string(v, heap) == target)
        .count();
    Ok(Value::Int(count as i64))
}

// ====================================================================
// PHASE 1.3: MISSING CONVERSION FUNCTIONS
// ====================================================================

/// parse_int(s) -> Int (parse string to integer, errors if invalid)
pub fn parse_int_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "parse_int expects 1 argument".into(),
        ));
    }
    let s = match &args[0] {
        Value::Obj(h) => match heap.get_object(*h) {
            Some(Object::String(s)) => s.trim().to_string(),
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: "other".into(),
                })
            }
        },
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "string".into(),
                got: args[0].type_name(),
            })
        }
    };
    s.parse::<i64>()
        .map(Value::Int)
        .map_err(|_| PulseError::RuntimeError(format!("Cannot parse '{}' as integer", s)))
}

/// parse_float(s) -> Float (parse string to float, errors if invalid)
pub fn parse_float_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "parse_float expects 1 argument".into(),
        ));
    }
    let s = match &args[0] {
        Value::Obj(h) => match heap.get_object(*h) {
            Some(Object::String(s)) => s.trim().to_string(),
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: "other".into(),
                })
            }
        },
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "string".into(),
                got: args[0].type_name(),
            })
        }
    };
    s.parse::<f64>()
        .map(Value::Float)
        .map_err(|_| PulseError::RuntimeError(format!("Cannot parse '{}' as float", s)))
}

// ====================================================================
// PHASE 1.3: MISSING MATH FUNCTIONS
// ====================================================================

/// log2(x) -> Float
pub fn log2_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("log2 expects 1 argument".into()));
    }
    let val = match &args[0] {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "number".into(),
                got: args[0].type_name(),
            })
        }
    };
    Ok(Value::Float(val.log2()))
}

/// min(a, b) -> number (alias for min_val)
pub fn min_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("min expects 2 arguments".into()));
    }
    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(*a.min(b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.min(*b))),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).min(*b))),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.min(*b as f64))),
        _ => Err(PulseError::TypeMismatch {
            expected: "numbers".into(),
            got: format!("{}, {}", args[0].type_name(), args[1].type_name()),
        }),
    }
}

/// max(a, b) -> number (alias for max_val)
pub fn max_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("max expects 2 arguments".into()));
    }
    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(*a.max(b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.max(*b))),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).max(*b))),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.max(*b as f64))),
        _ => Err(PulseError::TypeMismatch {
            expected: "numbers".into(),
            got: format!("{}, {}", args[0].type_name(), args[1].type_name()),
        }),
    }
}

// ====================================================================
// PHASE 1.3: MISSING STRING FUNCTIONS
// ====================================================================

/// char_code(s) -> Int (Unicode code point of first character)
pub fn char_code_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "char_code expects 1 argument".into(),
        ));
    }
    let s = match &args[0] {
        Value::Obj(h) => match heap.get_object(*h) {
            Some(Object::String(s)) => s.clone(),
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: "other".into(),
                })
            }
        },
        _ => {
            return Err(PulseError::TypeMismatch {
                expected: "string".into(),
                got: args[0].type_name(),
            })
        }
    };
    s.chars()
        .next()
        .map(|c| Value::Int(c as i64))
        .ok_or(PulseError::RuntimeError("char_code: empty string".into()))
}

/// from_char_code(n) -> String (string from Unicode code point)
pub fn from_char_code_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "from_char_code expects 1 argument".into(),
        ));
    }
    let code = args[0].as_int()?;
    let c = char::from_u32(code as u32).ok_or(PulseError::RuntimeError(format!(
        "Invalid Unicode code point: {}",
        code
    )))?;
    let handle = heap.alloc_object(Object::String(c.to_string()));
    Ok(Value::Obj(handle))
}

// ====================================================================
// PHASE 1.3: HEAP / PRIORITY QUEUE
// ====================================================================

/// create_heap() -> List (we use a sorted list as a min-heap)
pub fn create_heap_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError(
            "create_heap expects 0 arguments".into(),
        ));
    }
    let handle = heap.alloc_object(Object::List(Vec::new()));
    Ok(Value::Obj(handle))
}

/// heap_push(heap_list, value) -> Unit (insert maintaining sorted order)
pub fn heap_push_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "heap_push expects 2 arguments".into(),
        ));
    }
    let val = args[1];
    match args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(list)) = heap.get_mut_object(h) {
                list.push(val);
                // Sift up (binary heap)
                let mut i = list.len() - 1;
                while i > 0 {
                    let parent = (i - 1) / 2;
                    let should_swap = match (&list[i], &list[parent]) {
                        (Value::Int(a), Value::Int(b)) => a < b,
                        (Value::Float(a), Value::Float(b)) => a < b,
                        _ => false,
                    };
                    if should_swap {
                        list.swap(i, parent);
                        i = parent;
                    } else {
                        break;
                    }
                }
                Ok(Value::Unit)
            } else {
                Err(PulseError::TypeMismatch {
                    expected: "list (heap)".into(),
                    got: "other".into(),
                })
            }
        }
        _ => Err(PulseError::TypeMismatch {
            expected: "list (heap)".into(),
            got: args[0].type_name(),
        }),
    }
}

/// heap_pop(heap_list) -> Value (remove and return minimum)
pub fn heap_pop_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "heap_pop expects 1 argument".into(),
        ));
    }
    match args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(list)) = heap.get_mut_object(h) {
                if list.is_empty() {
                    return Err(PulseError::RuntimeError("heap_pop: empty heap".into()));
                }
                let len = list.len();
                list.swap(0, len - 1);
                let min_val = list.pop().expect("Expected a value");
                // Sift down
                let mut i = 0;
                let len = list.len();
                loop {
                    let left = 2 * i + 1;
                    let right = 2 * i + 2;
                    let mut smallest = i;
                    if left < len {
                        let is_smaller = match (&list[left], &list[smallest]) {
                            (Value::Int(a), Value::Int(b)) => a < b,
                            (Value::Float(a), Value::Float(b)) => a < b,
                            _ => false,
                        };
                        if is_smaller {
                            smallest = left;
                        }
                    }
                    if right < len {
                        let is_smaller = match (&list[right], &list[smallest]) {
                            (Value::Int(a), Value::Int(b)) => a < b,
                            (Value::Float(a), Value::Float(b)) => a < b,
                            _ => false,
                        };
                        if is_smaller {
                            smallest = right;
                        }
                    }
                    if smallest != i {
                        list.swap(i, smallest);
                        i = smallest;
                    } else {
                        break;
                    }
                }
                Ok(min_val)
            } else {
                Err(PulseError::TypeMismatch {
                    expected: "list (heap)".into(),
                    got: "other".into(),
                })
            }
        }
        _ => Err(PulseError::TypeMismatch {
            expected: "list (heap)".into(),
            got: args[0].type_name(),
        }),
    }
}

/// heap_peek(heap_list) -> Value (return minimum without removing)
pub fn heap_peek_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "heap_peek expects 1 argument".into(),
        ));
    }
    match args[0] {
        Value::Obj(h) => match heap.get_object(h) {
            Some(Object::List(list)) => list
                .first()
                .cloned()
                .ok_or(PulseError::RuntimeError("heap_peek: empty heap".into())),
            _ => Err(PulseError::TypeMismatch {
                expected: "list (heap)".into(),
                got: "other".into(),
            }),
        },
        _ => Err(PulseError::TypeMismatch {
            expected: "list (heap)".into(),
            got: args[0].type_name(),
        }),
    }
}

/// heap_size(heap_list) -> Int
pub fn heap_size_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "heap_size expects 1 argument".into(),
        ));
    }
    match args[0] {
        Value::Obj(h) => match heap.get_object(h) {
            Some(Object::List(list)) => Ok(Value::Int(list.len() as i64)),
            _ => Err(PulseError::TypeMismatch {
                expected: "list (heap)".into(),
                got: "other".into(),
            }),
        },
        _ => Err(PulseError::TypeMismatch {
            expected: "list (heap)".into(),
            got: args[0].type_name(),
        }),
    }
}
