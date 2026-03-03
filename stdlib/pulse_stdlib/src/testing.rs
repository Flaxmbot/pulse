//! Test framework native functions

use pulse_ast::object::{HeapInterface, Object};
use pulse_ast::{PulseError, PulseResult, Value};

/// assert(condition: Bool) -> Unit
/// Throws if condition is false
pub fn assert_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 1 {
        return Err(PulseError::RuntimeError("assert expects 1 argument".into()));
    }

    match &args[0] {
        Value::Bool(true) => Ok(Value::Unit),
        Value::Bool(false) => Err(PulseError::RuntimeError("Assertion failed".into())),
        _ => Err(PulseError::TypeMismatch {
            expected: "Bool".into(),
            got: args[0].type_name(),
        }),
    }
}

/// assert_eq(actual: Any, expected: Any) -> Unit
/// Throws if actual != expected
pub fn assert_eq_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "assert_eq expects 2 arguments".into(),
        ));
    }

    let equal = values_equal(&args[0], &args[1], heap);
    if equal {
        Ok(Value::Unit)
    } else {
        let actual_str = value_to_string(&args[0], heap);
        let expected_str = value_to_string(&args[1], heap);
        Err(PulseError::RuntimeError(format!(
            "Assertion failed: {} != {}",
            actual_str, expected_str
        )))
    }
}

/// assert_ne(actual: Any, expected: Any) -> Unit
/// Throws if actual == expected
pub fn assert_ne_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() != 2 {
        return Err(PulseError::RuntimeError(
            "assert_ne expects 2 arguments".into(),
        ));
    }

    let equal = values_equal(&args[0], &args[1], heap);
    if !equal {
        Ok(Value::Unit)
    } else {
        let value_str = value_to_string(&args[0], heap);
        Err(PulseError::RuntimeError(format!(
            "Assertion failed: values are equal: {}",
            value_str
        )))
    }
}

/// fail(message: String) -> Never
/// Always throws with message
pub fn fail_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    let msg = if args.is_empty() {
        "Test failed".to_string()
    } else {
        value_to_string(&args[0], heap)
    };
    Err(PulseError::RuntimeError(msg))
}

fn values_equal(a: &Value, b: &Value, heap: &dyn HeapInterface) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => (x - y).abs() < f64::EPSILON,
        (Value::Int(x), Value::Float(y)) | (Value::Float(y), Value::Int(x)) => {
            (*x as f64 - y).abs() < f64::EPSILON
        }
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Unit, Value::Unit) => true,
        (Value::Pid(x), Value::Pid(y)) => x == y,
        (Value::Obj(h1), Value::Obj(h2)) => {
            if h1 == h2 {
                return true;
            }
            match (heap.get_object(*h1), heap.get_object(*h2)) {
                (Some(Object::String(s1)), Some(Object::String(s2))) => s1 == s2,
                (Some(Object::List(l1)), Some(Object::List(l2))) => {
                    if l1.len() != l2.len() {
                        return false;
                    }
                    l1.iter()
                        .zip(l2.iter())
                        .all(|(a, b)| values_equal(a, b, heap))
                }
                _ => false,
            }
        }
        _ => false,
    }
}

fn value_to_string(val: &Value, heap: &dyn HeapInterface) -> String {
    match val {
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Unit => "nil".to_string(),
        Value::Pid(id) => format!("<pid:{:?}>", id),
        Value::Obj(h) => match heap.get_object(*h) {
            Some(Object::String(s)) => format!("\"{}\"", s),
            Some(Object::List(list)) => {
                let items: Vec<String> = list
                    .iter()
                    .take(5)
                    .map(|v| value_to_string(v, heap))
                    .collect();
                if list.len() > 5 {
                    format!("[{}, ...]", items.join(", "))
                } else {
                    format!("[{}]", items.join(", "))
                }
            }
            Some(Object::Map(_)) => "<map>".to_string(),
            Some(Object::Closure(_)) => "<fn>".to_string(),
            Some(Object::Instance(i)) => format!("<instance {}>", i.class.name),
            Some(Object::BoundMethod(_)) => "<bound method>".to_string(),
            _ => "<object>".to_string(),
        },
    }
}
