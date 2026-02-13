//! JSON native functions

use pulse_core::{Value, PulseResult, PulseError};
use pulse_core::object::{HeapInterface, Object};
use std::collections::HashMap;

/// json_parse(str: String) -> Map
/// Parses a JSON string into a Pulse value
pub fn json_parse_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("json_parse expects 1 argument".into()));
    }
    
    let json_str = match &args[0] {
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
    
    parse_json_value(&json_str.trim(), heap)
}

fn parse_json_value(s: &str, heap: &mut dyn HeapInterface) -> PulseResult<Value> {
    let s = s.trim();
    
    if s.starts_with('{') {
        parse_json_object(s, heap)
    } else if s.starts_with('[') {
        parse_json_array(s, heap)
    } else if s.starts_with('"') {
        parse_json_string(s, heap)
    } else if s == "true" {
        Ok(Value::Bool(true))
    } else if s == "false" {
        Ok(Value::Bool(false))
    } else if s == "null" {
        Ok(Value::Unit)
    } else if let Ok(i) = s.parse::<i64>() {
        Ok(Value::Int(i))
    } else if let Ok(f) = s.parse::<f64>() {
        Ok(Value::Float(f))
    } else {
        Err(PulseError::RuntimeError(format!("Invalid JSON: {}", s)))
    }
}

fn parse_json_object(s: &str, heap: &mut dyn HeapInterface) -> PulseResult<Value> {
    let s = s.trim();
    if !s.starts_with('{') || !s.ends_with('}') {
        return Err(PulseError::RuntimeError("Invalid JSON object".into()));
    }
    
    let inner = &s[1..s.len()-1].trim();
    if inner.is_empty() {
        let handle = heap.alloc_object(Object::Map(HashMap::new()));
        return Ok(Value::Obj(handle));
    }
    
    let mut map: HashMap<String, Value> = HashMap::new();
    let mut depth = 0;
    let mut in_string = false;
    let mut escape = false;
    let mut start = 0;
    
    let chars: Vec<char> = inner.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        let c = chars[i];
        
        if escape {
            escape = false;
            i += 1;
            continue;
        }
        
        match c {
            '\\' if in_string => escape = true,
            '"' => in_string = !in_string,
            '{' | '[' if !in_string => depth += 1,
            '}' | ']' if !in_string => depth -= 1,
            ',' if !in_string && depth == 0 => {
                let pair: String = chars[start..i].iter().collect();
                parse_kv_pair(&pair, &mut map, heap)?;
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    
    // Last pair
    if start < chars.len() {
        let pair: String = chars[start..].iter().collect();
        parse_kv_pair(&pair, &mut map, heap)?;
    }
    
    let handle = heap.alloc_object(Object::Map(map));
    Ok(Value::Obj(handle))
}

fn parse_kv_pair(s: &str, map: &mut HashMap<String, Value>, heap: &mut dyn HeapInterface) -> PulseResult<()> {
    let s = s.trim();
    if let Some(colon_pos) = find_colon_outside_quotes(s) {
        let key_str = s[..colon_pos].trim();
        let val_str = s[colon_pos+1..].trim();
        
        // Parse key (should be a quoted string)
        let key = parse_string_literal(key_str)?;
        let value = parse_json_value(val_str, heap)?;
        
        map.insert(key, value);
    }
    Ok(())
}

fn find_colon_outside_quotes(s: &str) -> Option<usize> {
    let mut in_string = false;
    let mut escape = false;
    
    for (i, c) in s.chars().enumerate() {
        if escape {
            escape = false;
            continue;
        }
        match c {
            '\\' if in_string => escape = true,
            '"' => in_string = !in_string,
            ':' if !in_string => return Some(i),
            _ => {}
        }
    }
    None
}

fn parse_string_literal(s: &str) -> PulseResult<String> {
    let s = s.trim();
    if !s.starts_with('"') || !s.ends_with('"') {
        return Err(PulseError::RuntimeError(format!("Invalid string: {}", s)));
    }
    
    let inner = &s[1..s.len()-1];
    let unescaped = inner.replace("\\\"", "\"")
                        .replace("\\n", "\n")
                        .replace("\\t", "\t")
                        .replace("\\\\", "\\");
    Ok(unescaped)
}

fn parse_json_array(s: &str, heap: &mut dyn HeapInterface) -> PulseResult<Value> {
    let s = s.trim();
    if !s.starts_with('[') || !s.ends_with(']') {
        return Err(PulseError::RuntimeError("Invalid JSON array".into()));
    }
    
    let inner = &s[1..s.len()-1].trim();
    if inner.is_empty() {
        let handle = heap.alloc_object(Object::List(Vec::new()));
        return Ok(Value::Obj(handle));
    }
    
    let mut list = Vec::new();
    let mut depth = 0;
    let mut in_string = false;
    let mut escape = false;
    let mut start = 0;
    
    let chars: Vec<char> = inner.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        let c = chars[i];
        
        if escape {
            escape = false;
            i += 1;
            continue;
        }
        
        match c {
            '\\' if in_string => escape = true,
            '"' => in_string = !in_string,
            '{' | '[' if !in_string => depth += 1,
            '}' | ']' if !in_string => depth -= 1,
            ',' if !in_string && depth == 0 => {
                let elem: String = chars[start..i].iter().collect();
                list.push(parse_json_value(&elem, heap)?);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    
    // Last element
    if start < chars.len() {
        let elem: String = chars[start..].iter().collect();
        list.push(parse_json_value(&elem, heap)?);
    }
    
    let handle = heap.alloc_object(Object::List(list));
    Ok(Value::Obj(handle))
}

fn parse_json_string(s: &str, heap: &mut dyn HeapInterface) -> PulseResult<Value> {
    let s = s.trim();
    if !s.starts_with('"') || !s.ends_with('"') {
        return Err(PulseError::RuntimeError(format!("Invalid JSON string: {}", s)));
    }
    
    let inner = &s[1..s.len()-1];
    let unescaped = inner.replace("\\\"", "\"")
                        .replace("\\n", "\n")
                        .replace("\\t", "\t")
                        .replace("\\\\", "\\");
    
    let handle = heap.alloc_object(Object::String(unescaped));
    Ok(Value::Obj(handle))
}

/// json_stringify(val: Value) -> String
/// Converts a Pulse value to JSON string
pub fn json_stringify_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("json_stringify expects 1 argument".into()));
    }
    
    let json_str = stringify_value(&args[0], heap)?;
    let handle = heap.alloc_object(Object::String(json_str));
    Ok(Value::Obj(handle))
}

fn stringify_value(val: &Value, heap: &dyn HeapInterface) -> PulseResult<String> {
    match val {
        Value::Int(i) => Ok(i.to_string()),
        Value::Float(f) => Ok(f.to_string()),
        Value::Bool(b) => Ok(if *b { "true" } else { "false" }.to_string()),
        Value::Unit => Ok("null".to_string()),
        Value::Obj(h) => {
            if let Some(obj) = heap.get_object(*h) {
                match obj {
                    Object::String(s) => Ok(format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))),
                    Object::List(list) => {
                        let items: Result<Vec<String>, _> = list.iter()
                            .map(|v| stringify_value(v, heap))
                            .collect();
                        Ok(format!("[{}]", items?.join(",")))
                    }
                    Object::Map(map) => {
                        let mut pairs = Vec::new();
                        for (k, v) in map {
                            // k is &String, v is &Value
                            let key_json = format!("\"{}\"", k.replace('\\', "\\\\").replace('"', "\\\""));
                            let val_json = stringify_value(v, heap)?;
                            pairs.push(format!("{}:{}", key_json, val_json));
                        }
                        Ok(format!("{{{}}}", pairs.join(",")))
                    }
                    Object::SharedMemory(_) => Ok("<shared_memory>".to_string()),
                    Object::Socket(_) => Ok("<socket>".to_string()),
                    _ => Ok("null".to_string()),
                }
            } else {
                Ok("null".to_string())
            }
        }
        Value::Pid(_) => Ok("null".to_string()),
    }
}
