//! Additional string manipulation native functions

use pulse_core::{Value, PulseResult, PulseError};
use pulse_core::object::{HeapInterface, Object};

/// split_string(str: String, delimiter: String) -> List
/// Splits a string by the given delimiter
pub fn split_string_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("split_string expects 2 arguments".into()));
    }

    let input = match &args[0] {
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

    let delimiter = match &args[1] {
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

    let parts: Vec<Value> = input.split(&delimiter)
        .map(|part| {
            let handle = heap.alloc_object(Object::String(part.to_string()));
            Value::Obj(handle)
        })
        .collect();

    let handle = heap.alloc_object(Object::List(parts));
    Ok(Value::Obj(handle))
}

/// join_strings(list: List, separator: String) -> String
/// Joins a list of strings with the given separator
pub fn join_strings_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("join_strings expects 2 arguments".into()));
    }

    let list_handle = match args[0] {
        Value::Obj(handle) => handle,
        _ => return Err(PulseError::TypeMismatch{
            expected: "list".into(),
            got: args[0].type_name()
        }),
    };

    let separator = match &args[1] {
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

    // Get the list
    let list_obj = heap.get_object(list_handle).ok_or(PulseError::RuntimeError("Invalid handle".into()))?;
    let list_values = if let Object::List(ref vals) = list_obj {
        vals.clone()
    } else {
        return Err(PulseError::TypeMismatch{expected: "list".into(), got: "other".into()});
    };

    let string_parts: Result<Vec<String>, _> = list_values.iter().map(|val| {
        match val {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    Ok(s.clone())
                } else {
                    Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into()
                    })
                }
            }
            _ => Err(PulseError::TypeMismatch {
                expected: "string".into(),
                got: val.type_name()
            }),
        }
    }).collect();

    let string_parts = string_parts?;
    let joined = string_parts.join(&separator);
    
    let handle = heap.alloc_object(Object::String(joined));
    Ok(Value::Obj(handle))
}

/// starts_with(str: String, prefix: String) -> Bool
/// Checks if a string starts with the given prefix
pub fn starts_with_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("starts_with expects 2 arguments".into()));
    }

    let input = match &args[0] {
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

    let prefix = match &args[1] {
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

    Ok(Value::Bool(input.starts_with(&prefix)))
}

/// ends_with(str: String, suffix: String) -> Bool
/// Checks if a string ends with the given suffix
pub fn ends_with_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("ends_with expects 2 arguments".into()));
    }

    let input = match &args[0] {
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

    let suffix = match &args[1] {
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

    Ok(Value::Bool(input.ends_with(&suffix)))
}

/// trim_string(str: String) -> String
/// Trims whitespace from both ends of a string
pub fn trim_string_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("trim_string expects 1 argument".into()));
    }

    let input = match &args[0] {
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

    let trimmed = input.trim();
    let handle = heap.alloc_object(Object::String(trimmed.to_string()));
    Ok(Value::Obj(handle))
}

/// string_length(str: String) -> Int
/// Returns the length of a string
pub fn string_length_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("string_length expects 1 argument".into()));
    }

    let input = match &args[0] {
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

    Ok(Value::Int(input.len() as i64))
}

/// substring(str: String, start: Int, end: Int) -> String
/// Returns a substring from start to end indices
pub fn substring_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 3 {
        return Err(PulseError::RuntimeError("substring expects 3 arguments".into()));
    }

    let input = match &args[0] {
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

    let start = args[1].as_int()?;
    let end = args[2].as_int()?;

    if start < 0 || end < 0 || start > end || end > input.len() as i64 {
        return Err(PulseError::RuntimeError("Invalid indices for substring".into()));
    }

    let start_idx = start as usize;
    let end_idx = end as usize;
    
    let substr = &input[start_idx..end_idx];
    let handle = heap.alloc_object(Object::String(substr.to_string()));
    Ok(Value::Obj(handle))
}

/// string_contains(str: String, substr: String) -> Bool
/// Checks if a string contains a substring
pub fn string_contains_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("string_contains expects 2 arguments".into()));
    }

    let input = match &args[0] {
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

    let substr = match &args[1] {
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

    Ok(Value::Bool(input.contains(&substr)))
}

/// string_replace(str: String, old: String, new: String) -> String
/// Replaces all occurrences of old substring with new substring
pub fn string_replace_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 3 {
        return Err(PulseError::RuntimeError("string_replace expects 3 arguments".into()));
    }

    let input = match &args[0] {
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

    let old = match &args[1] {
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

    let new = match &args[2] {
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

    let result = input.replace(&old, &new);
    let handle = heap.alloc_object(Object::String(result));
    Ok(Value::Obj(handle))
}

/// string_uppercase(str: String) -> String
/// Converts a string to uppercase
pub fn string_uppercase_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("string_uppercase expects 1 argument".into()));
    }

    let input = match &args[0] {
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

    let upper = input.to_uppercase();
    let handle = heap.alloc_object(Object::String(upper));
    Ok(Value::Obj(handle))
}

/// string_lowercase(str: String) -> String
/// Converts a string to lowercase
pub fn string_lowercase_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("string_lowercase expects 1 argument".into()));
    }

    let input = match &args[0] {
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

    let lower = input.to_lowercase();
    let handle = heap.alloc_object(Object::String(lower));
    Ok(Value::Obj(handle))
}