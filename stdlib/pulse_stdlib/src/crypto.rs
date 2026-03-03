use pulse_ast::object::{HeapInterface, Object};
use pulse_ast::{PulseError, PulseResult, Value};
use ring::digest::{Context, SHA256};
use bincode;
use serde_json;

pub fn sha256_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.is_empty() {
        return Err(PulseError::RuntimeError("Expected data string for sha256".into()));
    }

    let data_str = if let Value::Obj(h) = args[0] {
        if let Some(Object::String(s)) = heap.get_object(h) {
            s.clone()
        } else {
            return Err(PulseError::RuntimeError("Expected string data".into()));
        }
    } else {
        return Err(PulseError::RuntimeError("Expected string data".into()));
    };

    let mut context = Context::new(&SHA256);
    context.update(data_str.as_bytes());
    let digest = context.finish();

    // Convert to hex string
    let hex_str: String = digest.as_ref().iter().map(|b| format!("{:02x}", b)).collect();

    let handle = heap.alloc_object(Object::String(hex_str));
    Ok(Value::Obj(handle))
}

fn value_to_serde_value(heap: &dyn HeapInterface, val: &Value) -> Result<serde_json::Value, PulseError> {
    match val {
        Value::Bool(b) => Ok(serde_json::Value::Bool(*b)),
        Value::Int(i) => Ok(serde_json::Value::Number((*i).into())),
        Value::Float(f) => Ok(serde_json::Value::Number(serde_json::Number::from_f64(*f).unwrap_or(0.into()))),
        Value::Unit => Ok(serde_json::Value::Null),
        Value::Obj(h) => {
            if let Some(obj) = heap.get_object(*h) {
                match obj {
                    Object::String(s) => Ok(serde_json::Value::String(s.clone())),
                    Object::List(l) => {
                        let mut vec = Vec::new();
                        for item in l {
                            vec.push(value_to_serde_value(heap, item)?);
                        }
                        Ok(serde_json::Value::Array(vec))
                    }
                    Object::Map(m) => {
                        let mut map = serde_json::Map::new();
                        for (k, v) in m {
                            map.insert(k.clone(), value_to_serde_value(heap, v)?);
                        }
                        Ok(serde_json::Value::Object(map))
                    }
                    _ => Err(PulseError::RuntimeError("Cannot serialize complex object".into())),
                }
            } else {
                Err(PulseError::RuntimeError("Invalid handle".into()))
            }
        },
        _ => Err(PulseError::RuntimeError("Cannot serialize value".into())),
    }
}

fn serde_value_to_value(heap: &mut dyn HeapInterface, sval: &serde_json::Value) -> Result<Value, PulseError> {
    match sval {
        serde_json::Value::Null => Ok(Value::Unit),
        serde_json::Value::Bool(b) => Ok(Value::Bool(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Int(i))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::Float(f))
            } else {
                Ok(Value::Unit)
            }
        }
        serde_json::Value::String(s) => {
            let handle = heap.alloc_object(Object::String(s.clone()));
            Ok(Value::Obj(handle))
        }
        serde_json::Value::Array(a) => {
            let mut list = Vec::new();
            for item in a {
                list.push(serde_value_to_value(heap, item)?);
            }
            let handle = heap.alloc_object(Object::List(list));
            Ok(Value::Obj(handle))
        }
        serde_json::Value::Object(o) => {
            let mut map = std::collections::HashMap::new();
            for (k, v) in o {
                map.insert(k.clone(), serde_value_to_value(heap, v)?);
            }
            let handle = heap.alloc_object(Object::Map(map));
            Ok(Value::Obj(handle))
        }
    }
}

pub fn bincode_serialize_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.is_empty() {
        return Err(PulseError::RuntimeError("Expected value to serialize".into()));
    }

    // We intermediate via serde_json::Value for dynamic generic translation for now
    let sval = value_to_serde_value(heap, &args[0])?;
    match bincode::serialize(&sval) {
        Ok(bytes) => {
            let list: Vec<Value> = bytes.into_iter().map(|b: u8| Value::Int(b as i64)).collect();
            let handle = heap.alloc_object(Object::List(list));
            Ok(Value::Obj(handle))
        }
        Err(e) => Err(PulseError::RuntimeError(format!("Serialize failed: {}", e))),
    }
}

pub fn bincode_deserialize_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.is_empty() {
        return Err(PulseError::RuntimeError("Expected list of bytes".into()));
    }

    let byte_list = if let Value::Obj(h) = args[0] {
        if let Some(Object::List(l)) = heap.get_object(h) {
            let mut bytes = Vec::new();
            for v in l {
                if let Value::Int(i) = v {
                    bytes.push(*i as u8);
                } else {
                    return Err(PulseError::RuntimeError("Byte list must contain integers".into()));
                }
            }
            bytes
        } else {
            return Err(PulseError::RuntimeError("Expected list of bytes".into()));
        }
    } else {
        return Err(PulseError::RuntimeError("Expected list of bytes".into()));
    };

    match bincode::deserialize::<serde_json::Value>(&byte_list) {
        Ok(sval) => serde_value_to_value(heap, &sval),
        Err(e) => Err(PulseError::RuntimeError(format!("Deserialize failed: {}", e))),
    }
}
