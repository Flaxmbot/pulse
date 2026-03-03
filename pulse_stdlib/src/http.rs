//! HTTP client native functions with full HTTP capabilities

use once_cell::sync::Lazy;
use pulse_core::object::{HeapInterface, Object};
use pulse_core::{PulseError, PulseResult, Value};
use std::collections::HashMap;
use std::sync::Mutex;

// Global HTTP client instance (using reqwest with rustls for HTTPS support)
static HTTP_CLIENT: Lazy<reqwest::blocking::Client> = Lazy::new(|| {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client")
});

// Keep track of response bodies for streaming
static RESPONSE_BODIES: Lazy<Mutex<HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// http_get(url: String, headers: Map) -> Map
/// Performs an HTTP GET request
pub fn http_get_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.is_empty() {
        return Err(PulseError::RuntimeError(
            "http_get expects at least 1 argument".into(),
        ));
    }

    let url = extract_string(heap, &args[0])?;
    let headers = if args.len() > 1 {
        extract_map(heap, &args[1])?
    } else {
        HashMap::new()
    };

    let mut request = HTTP_CLIENT.get(&url);

    // Add headers
    for (key, val) in headers {
        if let Some(s) = val {
            request = request.header(&key, &s);
        }
    }

    let response = request
        .send()
        .map_err(|e| PulseError::RuntimeError(format!("HTTP GET failed: {}", e)))?;

    let status = response.status().as_u16();
    let mut response_headers = HashMap::new();

    for (key, val) in response.headers() {
        let key_str = key.to_string();
        let val_str = val.to_str().unwrap_or("").to_string();
        response_headers.insert(
            key_str,
            Value::Obj(heap.alloc_object(Object::String(val_str))),
        );
    }

    let body = response
        .text()
        .map_err(|e| PulseError::RuntimeError(format!("Failed to read response body: {}", e)))?;

    // Store body with a unique ID and return the ID
    let body_id = uuid::Uuid::new_v4().to_string();
    RESPONSE_BODIES
        .lock()
        .unwrap()
        .insert(body_id.clone(), body);

    let mut result = HashMap::new();
    result.insert("status".to_string(), Value::Int(status as i64));
    result.insert(
        "headers".to_string(),
        Value::Obj(heap.alloc_object(Object::Map(response_headers))),
    );
    result.insert(
        "body_id".to_string(),
        Value::Obj(heap.alloc_object(Object::String(body_id))),
    );
    result.insert(
        "body".to_string(),
        Value::Obj(heap.alloc_object(Object::String("".to_string()))),
    );

    Ok(Value::Obj(heap.alloc_object(Object::Map(result))))
}

/// http_post(url: String, body: String, headers: Map) -> Map
/// Performs an HTTP POST request
pub fn http_post_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() < 2 {
        return Err(PulseError::RuntimeError(
            "http_post expects at least 2 arguments".into(),
        ));
    }

    let url = extract_string(heap, &args[0])?;
    let body = extract_string(heap, &args[1])?;
    let headers = if args.len() > 2 {
        extract_map(heap, &args[2])?
    } else {
        HashMap::new()
    };

    let mut request = HTTP_CLIENT.post(&url).body(body);

    // Add headers
    for (key, val) in headers {
        if let Some(s) = val {
            request = request.header(&key, &s);
        }
    }

    let response = request
        .send()
        .map_err(|e| PulseError::RuntimeError(format!("HTTP POST failed: {}", e)))?;

    let status = response.status().as_u16();
    let mut response_headers = HashMap::new();

    for (key, val) in response.headers() {
        let key_str = key.to_string();
        let val_str = val.to_str().unwrap_or("").to_string();
        response_headers.insert(
            key_str,
            Value::Obj(heap.alloc_object(Object::String(val_str))),
        );
    }

    let resp_body = response
        .text()
        .map_err(|e| PulseError::RuntimeError(format!("Failed to read response body: {}", e)))?;

    let body_id = uuid::Uuid::new_v4().to_string();
    RESPONSE_BODIES
        .lock()
        .unwrap()
        .insert(body_id.clone(), resp_body);

    let mut result = HashMap::new();
    result.insert("status".to_string(), Value::Int(status as i64));
    result.insert(
        "headers".to_string(),
        Value::Obj(heap.alloc_object(Object::Map(response_headers))),
    );
    result.insert(
        "body_id".to_string(),
        Value::Obj(heap.alloc_object(Object::String(body_id))),
    );
    result.insert(
        "body".to_string(),
        Value::Obj(heap.alloc_object(Object::String("".to_string()))),
    );

    Ok(Value::Obj(heap.alloc_object(Object::Map(result))))
}

/// http_put(url: String, body: String, headers: Map) -> Map
/// Performs an HTTP PUT request
pub fn http_put_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() < 2 {
        return Err(PulseError::RuntimeError(
            "http_put expects at least 2 arguments".into(),
        ));
    }

    let url = extract_string(heap, &args[0])?;
    let body = extract_string(heap, &args[1])?;
    let headers = if args.len() > 2 {
        extract_map(heap, &args[2])?
    } else {
        HashMap::new()
    };

    let mut request = HTTP_CLIENT.put(&url).body(body);

    for (key, val) in headers {
        if let Some(s) = val {
            request = request.header(&key, &s);
        }
    }

    let response = request
        .send()
        .map_err(|e| PulseError::RuntimeError(format!("HTTP PUT failed: {}", e)))?;

    let status = response.status().as_u16();
    let mut response_headers = HashMap::new();

    for (key, val) in response.headers() {
        let key_str = key.to_string();
        let val_str = val.to_str().unwrap_or("").to_string();
        response_headers.insert(
            key_str,
            Value::Obj(heap.alloc_object(Object::String(val_str))),
        );
    }

    let resp_body = response
        .text()
        .map_err(|e| PulseError::RuntimeError(format!("Failed to read response body: {}", e)))?;

    let body_id = uuid::Uuid::new_v4().to_string();
    RESPONSE_BODIES
        .lock()
        .unwrap()
        .insert(body_id.clone(), resp_body);

    let mut result = HashMap::new();
    result.insert("status".to_string(), Value::Int(status as i64));
    result.insert(
        "headers".to_string(),
        Value::Obj(heap.alloc_object(Object::Map(response_headers))),
    );
    result.insert(
        "body_id".to_string(),
        Value::Obj(heap.alloc_object(Object::String(body_id))),
    );
    result.insert(
        "body".to_string(),
        Value::Obj(heap.alloc_object(Object::String("".to_string()))),
    );

    Ok(Value::Obj(heap.alloc_object(Object::Map(result))))
}

/// http_delete(url: String, headers: Map) -> Map
/// Performs an HTTP DELETE request
pub fn http_delete_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.is_empty() {
        return Err(PulseError::RuntimeError(
            "http_delete expects at least 1 argument".into(),
        ));
    }

    let url = extract_string(heap, &args[0])?;
    let headers = if args.len() > 1 {
        extract_map(heap, &args[1])?
    } else {
        HashMap::new()
    };

    let mut request = HTTP_CLIENT.delete(&url);

    for (key, val) in headers {
        if let Some(s) = val {
            request = request.header(&key, &s);
        }
    }

    let response = request
        .send()
        .map_err(|e| PulseError::RuntimeError(format!("HTTP DELETE failed: {}", e)))?;

    let status = response.status().as_u16();
    let mut response_headers = HashMap::new();

    for (key, val) in response.headers() {
        let key_str = key.to_string();
        let val_str = val.to_str().unwrap_or("").to_string();
        response_headers.insert(
            key_str,
            Value::Obj(heap.alloc_object(Object::String(val_str))),
        );
    }

    let resp_body = response
        .text()
        .map_err(|e| PulseError::RuntimeError(format!("Failed to read response body: {}", e)))?;

    let body_id = uuid::Uuid::new_v4().to_string();
    RESPONSE_BODIES
        .lock()
        .unwrap()
        .insert(body_id.clone(), resp_body);

    let mut result = HashMap::new();
    result.insert("status".to_string(), Value::Int(status as i64));
    result.insert(
        "headers".to_string(),
        Value::Obj(heap.alloc_object(Object::Map(response_headers))),
    );
    result.insert(
        "body_id".to_string(),
        Value::Obj(heap.alloc_object(Object::String(body_id))),
    );
    result.insert(
        "body".to_string(),
        Value::Obj(heap.alloc_object(Object::String("".to_string()))),
    );

    Ok(Value::Obj(heap.alloc_object(Object::Map(result))))
}

/// http_get_body(body_id: String) -> String
/// Retrieves the body content by ID
pub fn http_get_body_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "http_get_body expects 1 argument".into(),
        ));
    }

    let body_id = extract_string(heap, &args[0])?;

    let bodies = RESPONSE_BODIES.lock().unwrap();
    let body = bodies.get(&body_id).cloned().unwrap_or_default();

    Ok(Value::Obj(heap.alloc_object(Object::String(body))))
}

/// http_request(method: String, url: String, body: String, headers: Map) -> Map
/// Performs a generic HTTP request with any method
pub fn http_request_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() < 2 {
        return Err(PulseError::RuntimeError(
            "http_request expects at least 2 arguments".into(),
        ));
    }

    let method = extract_string(heap, &args[0])?.to_uppercase();
    let url = extract_string(heap, &args[1])?;
    let body = if args.len() > 2 {
        extract_string(heap, &args[2])?
    } else {
        String::new()
    };
    let headers = if args.len() > 3 {
        extract_map(heap, &args[3])?
    } else {
        HashMap::new()
    };

    let mut request = match method.as_str() {
        "GET" => HTTP_CLIENT.get(&url),
        "POST" => HTTP_CLIENT.post(&url).body(body),
        "PUT" => HTTP_CLIENT.put(&url).body(body),
        "DELETE" => HTTP_CLIENT.delete(&url),
        "PATCH" => HTTP_CLIENT.patch(&url).body(body),
        "HEAD" => HTTP_CLIENT.head(&url),
        _ => {
            return Err(PulseError::RuntimeError(format!(
                "Unsupported HTTP method: {}",
                method
            )))
        }
    };

    // Add headers
    for (key, val) in headers {
        if let Some(s) = val {
            request = request.header(&key, &s);
        }
    }

    let response = request
        .send()
        .map_err(|e| PulseError::RuntimeError(format!("HTTP request failed: {}", e)))?;

    let status = response.status().as_u16();
    let mut response_headers = HashMap::new();

    for (key, val) in response.headers() {
        let key_str = key.to_string();
        let val_str = val.to_str().unwrap_or("").to_string();
        response_headers.insert(
            key_str,
            Value::Obj(heap.alloc_object(Object::String(val_str))),
        );
    }

    let resp_body = response
        .text()
        .map_err(|e| PulseError::RuntimeError(format!("Failed to read response body: {}", e)))?;

    let body_id = uuid::Uuid::new_v4().to_string();
    RESPONSE_BODIES
        .lock()
        .unwrap()
        .insert(body_id.clone(), resp_body);

    let mut result = HashMap::new();
    result.insert("status".to_string(), Value::Int(status as i64));
    result.insert(
        "headers".to_string(),
        Value::Obj(heap.alloc_object(Object::Map(response_headers))),
    );
    result.insert(
        "body_id".to_string(),
        Value::Obj(heap.alloc_object(Object::String(body_id))),
    );
    result.insert(
        "method".to_string(),
        Value::Obj(heap.alloc_object(Object::String(method))),
    );
    result.insert(
        "body".to_string(),
        Value::Obj(heap.alloc_object(Object::String("".to_string()))),
    );

    Ok(Value::Obj(heap.alloc_object(Object::Map(result))))
}

// Helper functions

fn extract_string(heap: &dyn HeapInterface, value: &Value) -> Result<String, PulseError> {
    match value {
        Value::Obj(h) => {
            if let Some(Object::String(s)) = heap.get_object(*h) {
                Ok(s.clone())
            } else {
                Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: "object".into(),
                })
            }
        }
        _ => Err(PulseError::TypeMismatch {
            expected: "string".into(),
            got: value.type_name(),
        }),
    }
}

fn extract_map(
    heap: &dyn HeapInterface,
    value: &Value,
) -> Result<HashMap<String, Option<String>>, PulseError> {
    match value {
        Value::Obj(h) => {
            if let Some(Object::Map(m)) = heap.get_object(*h) {
                let mut result = HashMap::new();
                for (k, v) in m {
                    let val = match v {
                        Value::Obj(h) => {
                            if let Some(Object::String(s)) = heap.get_object(*h) {
                                Some(s.clone())
                            } else {
                                None
                            }
                        }
                        Value::Unit => None,
                        _ => Some(format!("{:?}", v)),
                    };
                    result.insert(k.clone(), val);
                }
                Ok(result)
            } else {
                Err(PulseError::TypeMismatch {
                    expected: "map".into(),
                    got: "object".into(),
                })
            }
        }
        _ => Err(PulseError::TypeMismatch {
            expected: "map".into(),
            got: value.type_name(),
        }),
    }
}

// Keep the original functions for compatibility
/// http_parse(raw: String) -> Map
/// Parses a raw HTTP request into a map
pub fn http_parse_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError(
            "http_parse expects 1 argument".into(),
        ));
    }

    let raw = extract_string(heap, &args[0])?;

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
        if line.is_empty() {
            break;
        }
        if let Some((key, val)) = line.split_once(':') {
            let handle = heap.alloc_object(Object::String(val.trim().to_string()));
            headers.insert(key.trim().to_string(), Value::Obj(handle));
        }
    }

    let mut map = HashMap::new();
    map.insert(
        "method".to_string(),
        Value::Obj(heap.alloc_object(Object::String(method))),
    );
    map.insert(
        "path".to_string(),
        Value::Obj(heap.alloc_object(Object::String(path))),
    );
    map.insert(
        "headers".to_string(),
        Value::Obj(heap.alloc_object(Object::Map(headers))),
    );

    let handle = heap.alloc_object(Object::Map(map));
    Ok(Value::Obj(handle))
}

/// http_format_response(status: Int, body: String) -> String
pub fn http_format_response_native(
    heap: &mut dyn HeapInterface,
    args: &[Value],
) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "http_format_response expects 2 arguments".into(),
        ));
    }

    let status = args[0].as_int()?;
    let body = extract_string(heap, &args[1])?;

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
