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
