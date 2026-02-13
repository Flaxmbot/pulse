//! Regular expression native functions

use pulse_core::{Value, PulseResult, PulseError};
use pulse_core::object::{HeapInterface, Object};
use regex::Regex;

/// regex_compile(pattern: String) -> Regex
/// Compiles a regular expression pattern
pub fn regex_compile_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("regex_compile expects 1 argument".into()));
    }

    let pattern = match &args[0] {
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

    match Regex::new(&pattern) {
        Ok(_regex) => {
            // For now, we'll store the pattern as a string since we can't store the Regex object directly
            // In a real implementation, we'd need to create a custom object type for Regex
            let handle = heap.alloc_object(Object::String(pattern));
            Ok(Value::Obj(handle))
        }
        Err(e) => Err(PulseError::RuntimeError(format!("Invalid regex pattern: {}", e))),
    }
}

/// regex_match(pattern: String, text: String) -> Bool
/// Checks if the text matches the pattern
pub fn regex_match_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("regex_match expects 2 arguments".into()));
    }

    let pattern = match &args[0] {
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

    let text = match &args[1] {
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

    match Regex::new(&pattern) {
        Ok(regex) => {
            let matches = regex.is_match(&text);
            Ok(Value::Bool(matches))
        }
        Err(e) => Err(PulseError::RuntimeError(format!("Invalid regex pattern: {}", e))),
    }
}

/// regex_find_all(pattern: String, text: String) -> List
/// Finds all matches of the pattern in the text
pub fn regex_find_all_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("regex_find_all expects 2 arguments".into()));
    }

    let pattern = match &args[0] {
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

    let text = match &args[1] {
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

    match Regex::new(&pattern) {
        Ok(regex) => {
            let matches: Vec<Value> = regex.find_iter(&text)
                .map(|mat| {
                    let handle = heap.alloc_object(Object::String(mat.as_str().to_string()));
                    Value::Obj(handle)
                })
                .collect();
            
            let handle = heap.alloc_object(Object::List(matches));
            Ok(Value::Obj(handle))
        }
        Err(e) => Err(PulseError::RuntimeError(format!("Invalid regex pattern: {}", e))),
    }
}

/// regex_replace(pattern: String, replacement: String, text: String) -> String
/// Replaces all matches of the pattern in the text with the replacement
pub fn regex_replace_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 3 {
        return Err(PulseError::RuntimeError("regex_replace expects 3 arguments".into()));
    }

    let pattern = match &args[0] {
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

    let replacement = match &args[1] {
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

    let text = match &args[2] {
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
            got: args[2].type_name()
        }),
    };

    match Regex::new(&pattern) {
        Ok(regex) => {
            let result = regex.replace_all(&text, &replacement).to_string();
            let handle = heap.alloc_object(Object::String(result));
            Ok(Value::Obj(handle))
        }
        Err(e) => Err(PulseError::RuntimeError(format!("Invalid regex pattern: {}", e))),
    }
}