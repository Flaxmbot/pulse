//! UUID generation native functions

use pulse_core::{Value, PulseResult, PulseError};
use pulse_core::object::{HeapInterface, Object};

/// uuid_generate() -> String
/// Generates a new UUID v4
pub fn uuid_generate_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError("uuid_generate takes no arguments".into()));
    }

    let uuid = uuid::Uuid::new_v4().to_string();
    Ok(Value::Obj(heap.alloc_object(Object::String(uuid))))
}

/// uuid_v4() -> String
/// Generates a new UUID v4 (alias for uuid_generate)
pub fn uuid_v4_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    uuid_generate_native(heap, args)
}

/// uuid_parse(uuid_str: String) -> Map
/// Parses a UUID string into its components
pub fn uuid_parse_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("uuid_parse expects 1 argument".into()));
    }

    let uuid_str = extract_string(heap, &args[0])?;

    let uuid = uuid::Uuid::parse_str(&uuid_str)
        .map_err(|e| PulseError::RuntimeError(format!("Invalid UUID: {}", e)))?;

    let mut map = std::collections::HashMap::new();
    
    // Get the variant
    let variant = match uuid.get_variant() {
        uuid::Variant::NCS => "NCS",
        uuid::Variant::RFC4122 => "RFC4122",
        uuid::Variant::Microsoft => "Microsoft",
        uuid::Variant::Future => "Future",
        _ => "Unknown",
    };
    map.insert("variant".to_string(), Value::Obj(heap.alloc_object(Object::String(variant.to_string()))));
    
    // Get the version
    let version = match uuid.get_version() {
        Some(uuid::Version::Md5) => 3,
        Some(uuid::Version::Random) => 4,
        Some(uuid::Version::Sha1) => 5,
        _ => 0,
    };
    map.insert("version".to_string(), Value::Int(version));
    
    // Get raw bytes
    let bytes = uuid.as_bytes();
    let bytes_str = bytes.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(":");
    map.insert("bytes".to_string(), Value::Obj(heap.alloc_object(Object::String(bytes_str))));
    
    // Get the simple string format
    map.insert("simple".to_string(), Value::Obj(heap.alloc_object(Object::String(uuid.simple().to_string()))));
    
    // Get the urn format
    map.insert("urn".to_string(), Value::Obj(heap.alloc_object(Object::String(uuid.urn().to_string()))));

    Ok(Value::Obj(heap.alloc_object(Object::Map(map))))
}

/// uuid_to_string(uuid_str: String) -> String
/// Converts a UUID to its string representation
pub fn uuid_to_string_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("uuid_to_string expects 1 argument".into()));
    }

    let uuid_str = extract_string(heap, &args[0])?;

    let _uuid = uuid::Uuid::parse_str(&uuid_str)
        .map_err(|e| PulseError::RuntimeError(format!("Invalid UUID: {}", e)))?;

    // Return the canonical string representation
    Ok(Value::Obj(heap.alloc_object(Object::String(uuid_str))))
}

/// uuid_is_valid(uuid_str: String) -> Bool
/// Checks if a string is a valid UUID
pub fn uuid_is_valid_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("uuid_is_valid expects 1 argument".into()));
    }

    let uuid_str = extract_string(heap, &args[0])?;

    let result = uuid::Uuid::parse_str(&uuid_str).is_ok();
    Ok(Value::Bool(result))
}

/// uuid_nil() -> String
/// Returns the nil UUID (all zeros)
pub fn uuid_nil_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError("uuid_nil takes no arguments".into()));
    }

    let uuid = uuid::Uuid::nil().to_string();
    Ok(Value::Obj(heap.alloc_object(Object::String(uuid))))
}

/// uuid_namespace_nsdns() -> String
/// Returns the NSDNS namespace UUID
pub fn uuid_namespace_ns_dns_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError("uuid_namespace_nsdns takes no arguments".into()));
    }

    let uuid = uuid::Uuid::NAMESPACE_DNS.to_string();
    Ok(Value::Obj(heap.alloc_object(Object::String(uuid))))
}

/// uuid_namespace_nsurl() -> String
/// Returns the NSURL namespace UUID
pub fn uuid_namespace_ns_url_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError("uuid_namespace_nsurl takes no arguments".into()));
    }

    let uuid = uuid::Uuid::NAMESPACE_URL.to_string();
    Ok(Value::Obj(heap.alloc_object(Object::String(uuid))))
}

/// uuid_namespace_nsoid() -> String
/// Returns the NSOID namespace UUID
pub fn uuid_namespace_ns_oid_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError("uuid_namespace_nsoid takes no arguments".into()));
    }

    let uuid = uuid::Uuid::NAMESPACE_OID.to_string();
    Ok(Value::Obj(heap.alloc_object(Object::String(uuid))))
}

/// uuid_namespace_x500() -> String
/// Returns the X500 namespace UUID
pub fn uuid_namespace_x500_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError("uuid_namespace_x500 takes no arguments".into()));
    }

    let uuid = uuid::Uuid::NAMESPACE_X500.to_string();
    Ok(Value::Obj(heap.alloc_object(Object::String(uuid))))
}

/// uuid_from_bytes(bytes: List) -> String
/// Creates a UUID from a list of bytes
pub fn uuid_from_bytes_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("uuid_from_bytes expects 1 argument".into()));
    }

    let bytes = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::List(l)) = heap.get_object(*h) {
                l.clone()
            } else {
                return Err(PulseError::TypeMismatch { expected: "list".into(), got: "object".into() });
            }
        }
        _ => return Err(PulseError::TypeMismatch { expected: "list".into(), got: args[0].type_name() }),
    };

    if bytes.len() != 16 {
        return Err(PulseError::RuntimeError("UUID requires exactly 16 bytes".into()));
    }

    let mut byte_array = [0u8; 16];
    for (i, byte_val) in bytes.iter().enumerate() {
        if let Value::Int(n) = byte_val {
            byte_array[i] = *n as u8;
        } else {
            return Err(PulseError::RuntimeError("Expected integer byte values".into()));
        }
    }

    // Convert bytes to UUID
    let uuid = uuid::Uuid::from_bytes(byte_array);
    
    Ok(Value::Obj(heap.alloc_object(Object::String(uuid.to_string()))))
}

// Helper function
fn extract_string(heap: &dyn HeapInterface, value: &Value) -> Result<String, PulseError> {
    match value {
        Value::Obj(h) => {
            if let Some(Object::String(s)) = heap.get_object(*h) {
                Ok(s.clone())
            } else {
                Err(PulseError::TypeMismatch { expected: "string".into(), got: "object".into() })
            }
        }
        _ => Err(PulseError::TypeMismatch { expected: "string".into(), got: value.type_name() }),
    }
}
