//! HTTP utility native functions

use pulse_core::{Value, PulseResult, PulseError};
use pulse_core::object::{HeapInterface, Object};
use std::collections::HashMap;

/// http_parse(raw: String) -> Map
/// Parses a raw HTTP request into a map
pub fn http_parse_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("http_parse expects 1 argument".into()));
    }

    let raw = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::String(s)) = heap.get_object(*h) {
                s.clone()
            } else {
                return Err(PulseError::TypeMismatch { expected: "string".into(), got: "object".into() });
            }
        }
        _ => return Err(PulseError::TypeMismatch { expected: "string".into(), got: args[0].type_name() }),
    };

    let mut lines = raw.lines();
    let first_line = match lines.next() {
        Some(l) => l,
        None => return Err(PulseError::RuntimeError("Empty request".into())),
    };
    
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(PulseError::RuntimeError("Invalid HTTP request line".into()));
    }

    let method = parts[0].to_string();
    let path = parts[1].to_string();

    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() { break; }
        if let Some((key, val)) = line.split_once(':') {
            let handle = heap.alloc_object(Object::String(val.trim().to_string()));
            headers.insert(key.trim().to_string(), Value::Obj(handle));
        }
    }

    let mut map = HashMap::new();
    map.insert("method".to_string(), Value::Obj(heap.alloc_object(Object::String(method))));
    map.insert("path".to_string(), Value::Obj(heap.alloc_object(Object::String(path))));
    map.insert("headers".to_string(), Value::Obj(heap.alloc_object(Object::Map(headers))));

    let handle = heap.alloc_object(Object::Map(map));
    Ok(Value::Obj(handle))
}

/// http_format_response(status: Int, body: String) -> String
pub fn http_format_response_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("http_format_response expects 2 arguments".into()));
    }

    let status = args[0].as_int()?;
    let body = match &args[1] {
        Value::Obj(h) => {
            if let Some(Object::String(s)) = heap.get_object(*h) {
                s.clone()
            } else {
                return Err(PulseError::TypeMismatch { expected: "string".into(), got: "object".into() });
            }
        }
        _ => return Err(PulseError::TypeMismatch { expected: "string".into(), got: args[1].type_name() }),
    };

    let status_text = match status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Unknown",
    };

    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n{}",
        status, status_text, body.len(), body
    );

    let handle = heap.alloc_object(Object::String(response));
    Ok(Value::Obj(handle))
}
