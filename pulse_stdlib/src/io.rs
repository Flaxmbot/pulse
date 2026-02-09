//! File I/O native functions

use pulse_core::{Value, PulseResult, PulseError};
use pulse_core::object::{HeapInterface, Object};
use std::fs;

/// read_file(path: String) -> String
/// Reads entire file contents as a string
pub fn read_file_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("read_file expects 1 argument".into()));
    }
    
    let path = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::String(s)) = heap.get_object(*h) {
                s.clone()
            } else {
                return Err(PulseError::TypeMismatch { 
                    expected: "string".into(), 
                    got: "object".into() 
                });
            }
        }
        _ => return Err(PulseError::TypeMismatch { 
            expected: "string".into(), 
            got: args[0].type_name() 
        }),
    };
    
    match fs::read_to_string(&path) {
        Ok(content) => {
            let handle = heap.alloc_object(Object::String(content));
            Ok(Value::Obj(handle))
        }
        Err(e) => Err(PulseError::RuntimeError(format!("Failed to read file '{}': {}", path, e))),
    }
}

/// write_file(path: String, content: String) -> Bool
/// Writes content to file, returns true on success
pub fn write_file_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("write_file expects 2 arguments".into()));
    }
    
    let path = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::String(s)) = heap.get_object(*h) {
                s.clone()
            } else {
                return Err(PulseError::TypeMismatch { 
                    expected: "string".into(), 
                    got: "object".into() 
                });
            }
        }
        _ => return Err(PulseError::TypeMismatch { 
            expected: "string".into(), 
            got: args[0].type_name() 
        }),
    };
    
    let content = match &args[1] {
        Value::Obj(h) => {
            if let Some(Object::String(s)) = heap.get_object(*h) {
                s.clone()
            } else {
                return Err(PulseError::TypeMismatch { 
                    expected: "string".into(), 
                    got: "object".into() 
                });
            }
        }
        _ => return Err(PulseError::TypeMismatch { 
            expected: "string".into(), 
            got: args[1].type_name() 
        }),
    };
    
    match fs::write(&path, &content) {
        Ok(_) => Ok(Value::Bool(true)),
        Err(_) => Ok(Value::Bool(false)),
    }
}

/// file_exists(path: String) -> Bool
/// Checks if a file exists
pub fn file_exists_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("file_exists expects 1 argument".into()));
    }
    
    let path = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::String(s)) = heap.get_object(*h) {
                s.clone()
            } else {
                return Err(PulseError::TypeMismatch { 
                    expected: "string".into(), 
                    got: "object".into() 
                });
            }
        }
        _ => return Err(PulseError::TypeMismatch { 
            expected: "string".into(), 
            got: args[0].type_name() 
        }),
    };
    
    Ok(Value::Bool(std::path::Path::new(&path).exists()))
}

/// delete_file(path: String) -> Bool
/// Deletes a file, returns true on success
pub fn delete_file_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("delete_file expects 1 argument".into()));
    }
    
    let path = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::String(s)) = heap.get_object(*h) {
                s.clone()
            } else {
                return Err(PulseError::TypeMismatch { 
                    expected: "string".into(), 
                    got: "object".into() 
                });
            }
        }
        _ => return Err(PulseError::TypeMismatch { 
            expected: "string".into(), 
            got: args[0].type_name() 
        }),
    };
    
    match fs::remove_file(&path) {
        Ok(_) => Ok(Value::Bool(true)),
        Err(_) => Ok(Value::Bool(false)),
    }
}
