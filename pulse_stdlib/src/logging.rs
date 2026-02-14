//! Logging native functions

use pulse_core::{Value, PulseResult, PulseError};
use pulse_core::object::{HeapInterface, Object};
use std::cell::Cell;
use chrono::Local;

/// Thread-local log level
thread_local! {
    static LOG_LEVEL: Cell<i32> = Cell::new(2); // 0=Debug, 1=Info, 2=Warn, 3=Error
    static LOG_ENABLED: Cell<bool> = Cell::new(true);
}

/// set_log_level(level: String) -> Unit
/// Sets the minimum log level (debug, info, warn, error)
pub fn set_log_level_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("set_log_level expects 1 argument".into()));
    }
    
    let level_str = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::String(s)) = _heap.get_object(*h) {
                s.clone()
            } else {
                return Err(PulseError::TypeMismatch { expected: "string".into(), got: "object".into() });
            }
        }
        _ => return Err(PulseError::TypeMismatch { expected: "string".into(), got: args[0].type_name() }),
    };
    
    let level = match level_str.to_lowercase().as_str() {
        "debug" => 0,
        "info" => 1,
        "warn" | "warning" => 2,
        "error" => 3,
        _ => 1,
    };
    
    LOG_LEVEL.with(|l| l.set(level));
    Ok(Value::Unit)
}

/// get_log_level() -> String
/// Gets the current log level
pub fn get_log_level_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError("get_log_level expects 0 arguments".into()));
    }
    
    let level = LOG_LEVEL.with(|l| l.get());
    let level_str = match level {
        0 => "debug",
        1 => "info",
        2 => "warn",
        3 => "error",
        _ => "info",
    };
    Ok(Value::Obj(_heap.alloc_object(Object::String(level_str.to_string()))))
}

/// debug(message: Any) -> Unit
/// Logs a debug message
pub fn debug_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    log_message(heap, args, 0, "DEBUG")
}

/// info(message: Any) -> Unit
/// Logs an info message
pub fn info_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    log_message(heap, args, 1, "INFO")
}

/// warn(message: Any) -> Unit
/// Logs a warning message
pub fn warn_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    log_message(heap, args, 2, "WARN")
}

/// error(message: Any) -> Unit
/// Logs an error message
pub fn error_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    log_message(heap, args, 3, "ERROR")
}

/// log(level: String, message: Any) -> Unit
/// Logs a message with the specified level
pub fn log_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("log expects 2 arguments".into()));
    }
    
    let level_str = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::String(s)) = heap.get_object(*h) {
                s.clone()
            } else {
                return Err(PulseError::TypeMismatch { expected: "string".into(), got: "object".into() });
            }
        }
        _ => return Err(PulseError::TypeMismatch { expected: "string".into(), got: args[0].type_name() }),
    };
    
    let level = match level_str.to_lowercase().as_str() {
        "debug" => 0,
        "info" => 1,
        "warn" | "warning" => 2,
        "error" => 3,
        _ => 1,
    };
    
    log_message(heap, &args[1..], level, &level_str.to_uppercase())
}

/// Internal function to log a message
fn log_message(heap: &mut dyn HeapInterface, args: &[Value], level: i32, level_str: &str) -> PulseResult<Value> {
    if !LOG_ENABLED.with(|e| e.get()) {
        return Ok(Value::Unit);
    }
    
    let min_level = LOG_LEVEL.with(|l| l.get());
    if level < min_level {
        return Ok(Value::Unit);
    }
    
    let message = format_value_to_string(heap, args)?;
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    let formatted = format!("[{}] [{}] {}", timestamp, level_str, message);
    
    match level {
        0 | 1 => {
            println!("{}", formatted);
        }
        _ => {
            eprintln!("{}", formatted);
        }
    }
    
    Ok(Value::Unit)
}

/// Formats a Pulse value to a string
fn format_value_to_string(heap: &dyn HeapInterface, args: &[Value]) -> PulseResult<String> {
    let mut parts = Vec::new();
    
    for arg in args {
        let s = value_to_string(heap, arg)?;
        parts.push(s);
    }
    
    Ok(parts.join(" "))
}

/// Converts a Pulse value to a string
fn value_to_string(heap: &dyn HeapInterface, value: &Value) -> PulseResult<String> {
    match value {
        Value::Int(i) => Ok(i.to_string()),
        Value::Float(f) => Ok(f.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Unit => Ok("unit".to_string()),
        Value::Pid(p) => Ok(format!("<actor {:?}>", p)),
        Value::Obj(h) => {
            if let Some(obj) = heap.get_object(*h) {
                match obj {
                    Object::String(s) => Ok(s.clone()),
                    Object::List(l) => {
                        let items: Vec<String> = l.iter()
                            .map(|v| value_to_string(heap, v))
                            .collect::<Result<Vec<_>, _>>()?;
                        Ok(format!("[{}]", items.join(", ")))
                    }
                    Object::Map(m) => {
                        let items: Vec<String> = m.iter()
                            .map(|(k, v)| {
                                let vs = value_to_string(heap, v)?;
                                Ok(format!("{}: {}", k, vs))
                            })
                            .collect::<Result<Vec<String>, _>>()?;
                        Ok(format!("{{{}}}", items.join(", ")))
                    }
                    Object::Closure(_) => Ok("<closure>".to_string()),
                    Object::Function(f) => Ok(format!("<fn {}>", f.name)),
                    Object::NativeFn(n) => Ok(format!("<native {}>", n.name)),
                    Object::Module(m) => Ok(format!("<module len={}>", m.len())),
                    Object::Class(c) => Ok(format!("<class {}>", c.name)),
                    Object::Instance(i) => Ok(format!("<instance {}>", i.class.name)),
                    Object::Regex(r) => Ok(format!("<regex {:?}>", r)),
                    _ => Ok("<object>".to_string()),
                }
            } else {
                Ok("<null>".to_string())
            }
        }
    }
}

/// set_log_format(_format: String) -> Unit
/// Sets the log message format (placeholder - not implemented)
pub fn set_log_format_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("set_log_format expects 1 argument".into()));
    }
    
    Ok(Value::Unit)
}

/// enable_logging() -> Unit
/// Enables logging
pub fn enable_logging_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError("enable_logging expects 0 arguments".into()));
    }
    
    LOG_ENABLED.with(|e| e.set(true));
    Ok(Value::Unit)
}

/// disable_logging() -> Unit
/// Disables logging
pub fn disable_logging_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError("disable_logging expects 0 arguments".into()));
    }
    
    LOG_ENABLED.with(|e| e.set(false));
    Ok(Value::Unit)
}

/// logging_enabled() -> Bool
/// Returns whether logging is enabled
pub fn logging_enabled_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if !args.is_empty() {
        return Err(PulseError::RuntimeError("logging_enabled expects 0 arguments".into()));
    }
    
    Ok(Value::Bool(LOG_ENABLED.with(|e| e.get())))
}

/// log_fatal(message: Any) -> Unit
/// Logs a fatal error message and panics
pub fn log_fatal_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    log_message(heap, args, 3, "FATAL")?;
    let message = format_value_to_string(heap, args)?;
    Err(PulseError::RuntimeError(format!("FATAL: {}", message)))
}

/// log_debug_if(condition: Bool, message: Any) -> Unit
/// Logs a debug message only if condition is true
pub fn log_debug_if_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("log_debug_if expects 2 arguments".into()));
    }
    
    let condition = args[0].as_bool()?;
    if condition {
        log_message(heap, &args[1..], 0, "DEBUG")?;
    }
    
    Ok(Value::Unit)
}

/// log_info_if(condition: Bool, message: Any) -> Unit
/// Logs an info message only if condition is true
pub fn log_info_if_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("log_info_if expects 2 arguments".into()));
    }
    
    let condition = args[0].as_bool()?;
    if condition {
        log_message(heap, &args[1..], 1, "INFO")?;
    }
    
    Ok(Value::Unit)
}

/// trace(message: Any) -> Unit
/// Logs a trace message (same as debug)
pub fn trace_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    log_message(heap, args, 0, "TRACE")
}

/// log_with_context(context: Map, message: Any) -> Unit
/// Logs a message with additional context
pub fn log_with_context_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError("log_with_context expects 2 arguments".into()));
    }
    
    let context = match &args[0] {
        Value::Obj(h) => {
            if let Some(Object::Map(m)) = heap.get_object(*h) {
                m.clone()
            } else {
                return Err(PulseError::TypeMismatch { expected: "map".into(), got: "object".into() });
            }
        }
        _ => return Err(PulseError::TypeMismatch { expected: "map".into(), got: args[0].type_name() }),
    };
    
    let message = format_value_to_string(heap, &args[1..])?;
    
    let context_str = context.iter()
        .map(|(k, v)| {
            let vs = value_to_string(heap, v)?;
            Ok(format!("{}={}", k, vs))
        })
        .collect::<Result<Vec<String>, _>>()?;
    
    let formatted = format!("{} [{}]", message, context_str.join(", "));
    
    if LOG_ENABLED.with(|e| e.get()) {
        println!("{}", formatted);
    }
    
    Ok(Value::Unit)
}
