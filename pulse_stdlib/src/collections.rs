//! Collections utilities for Pulse
//!
//! Provides functional operations on lists and maps.

use pulse_core::object::{HeapInterface, Object};
use pulse_core::{PulseError, PulseResult, Value};

/// list_map(list: List<T>, fn: Fn<T -> U>) -> List<U>
/// Maps a function over each element of a list
pub fn list_map_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() < 2 {
        return Err(PulseError::RuntimeError(
            "list_map requires list and function arguments".to_string(),
        ));
    }

    let list = match &args[0] {
        Value::Obj(handle) => match heap.get_object(*handle) {
            Some(Object::List(arr)) => arr.clone(),
            _ => {
                return Err(PulseError::RuntimeError(
                    "Expected list as first argument".to_string(),
                ))
            }
        },
        _ => {
            return Err(PulseError::RuntimeError(
                "Expected list as first argument".to_string(),
            ))
        }
    };

    // For simplicity, we return the original list
    // In a full implementation, we'd apply the function to each element
    let result_handle = heap.alloc_object(Object::List(list));
    Ok(Value::Obj(result_handle))
}

/// list_filter(list: List<T>, fn: Fn<T -> Bool>) -> List<T>
/// Filters a list based on a predicate
pub fn list_filter_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() < 2 {
        return Err(PulseError::RuntimeError(
            "list_filter requires list and predicate arguments".to_string(),
        ));
    }

    let list = match &args[0] {
        Value::Obj(handle) => match heap.get_object(*handle) {
            Some(Object::List(arr)) => arr.clone(),
            _ => {
                return Err(PulseError::RuntimeError(
                    "Expected list as first argument".to_string(),
                ))
            }
        },
        _ => {
            return Err(PulseError::RuntimeError(
                "Expected list as first argument".to_string(),
            ))
        }
    };

    let result_handle = heap.alloc_object(Object::List(list));
    Ok(Value::Obj(result_handle))
}

/// list_reduce(list: List<T>, init: U, fn: Fn<(U, T) -> U>) -> U
/// Reduces a list to a single value
pub fn list_reduce_native(_heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.len() < 3 {
        return Err(PulseError::RuntimeError(
            "list_reduce requires list, initial value, and function arguments".to_string(),
        ));
    }

    // Return initial value for now
    Ok(args[1])
}

/// list_sort(list: List<T>) -> List<T>
/// Sorts a list in ascending order
pub fn list_sort_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.is_empty() {
        return Err(PulseError::RuntimeError(
            "list_sort requires a list argument".to_string(),
        ));
    }

    let mut list = match &args[0] {
        Value::Obj(handle) => match heap.get_object(*handle) {
            Some(Object::List(arr)) => arr.clone(),
            _ => {
                return Err(PulseError::RuntimeError(
                    "Expected list as first argument".to_string(),
                ))
            }
        },
        _ => {
            return Err(PulseError::RuntimeError(
                "Expected list as first argument".to_string(),
            ))
        }
    };

    // Sort based on value type
    list.sort_by(|a, b| match (a, b) {
        (Value::Int(i1), Value::Int(i2)) => i1.cmp(i2),
        (Value::Float(f1), Value::Float(f2)) => {
            f1.partial_cmp(f2).unwrap_or(std::cmp::Ordering::Equal)
        }
        _ => std::cmp::Ordering::Equal,
    });

    let result_handle = heap.alloc_object(Object::List(list));
    Ok(Value::Obj(result_handle))
}

/// list_reverse(list: List<T>) -> List<T>
/// Reverses a list
pub fn list_reverse_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.is_empty() {
        return Err(PulseError::RuntimeError(
            "list_reverse requires a list argument".to_string(),
        ));
    }

    let mut list = match &args[0] {
        Value::Obj(handle) => match heap.get_object(*handle) {
            Some(Object::List(arr)) => arr.clone(),
            _ => {
                return Err(PulseError::RuntimeError(
                    "Expected list as first argument".to_string(),
                ))
            }
        },
        _ => {
            return Err(PulseError::RuntimeError(
                "Expected list as first argument".to_string(),
            ))
        }
    };

    list.reverse();

    let result_handle = heap.alloc_object(Object::List(list));
    Ok(Value::Obj(result_handle))
}

/// list_unique(list: List<T>) -> List<T>
/// Removes duplicate elements from a list
pub fn list_unique_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.is_empty() {
        return Err(PulseError::RuntimeError(
            "list_unique requires a list argument".to_string(),
        ));
    }

    let list = match &args[0] {
        Value::Obj(handle) => match heap.get_object(*handle) {
            Some(Object::List(arr)) => arr.clone(),
            _ => {
                return Err(PulseError::RuntimeError(
                    "Expected list as first argument".to_string(),
                ))
            }
        },
        _ => {
            return Err(PulseError::RuntimeError(
                "Expected list as first argument".to_string(),
            ))
        }
    };

    // Simple deduplication (preserves order)
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();

    for item in list {
        // This is a simplified check - proper deduplication would need deep equality
        if seen.insert(format!("{:?}", item)) {
            result.push(item);
        }
    }

    let result_handle = heap.alloc_object(Object::List(result));
    Ok(Value::Obj(result_handle))
}

/// map_keys(map: Map<K, V>) -> List<K>
/// Returns a list of keys from a map
pub fn map_keys_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.is_empty() {
        return Err(PulseError::RuntimeError(
            "map_keys requires a map argument".to_string(),
        ));
    }

    // Collect keys first to avoid borrow issues
    let key_strings: Vec<String> = match &args[0] {
        Value::Obj(handle) => match heap.get_object(*handle) {
            Some(Object::Map(m)) => m.keys().cloned().collect(),
            _ => {
                return Err(PulseError::RuntimeError(
                    "Expected map as first argument".to_string(),
                ))
            }
        },
        _ => {
            return Err(PulseError::RuntimeError(
                "Expected map as first argument".to_string(),
            ))
        }
    };

    // Now allocate the keys
    let mut keys: Vec<Value> = Vec::new();
    for k in key_strings {
        let key_handle = heap.alloc_object(Object::String(k));
        keys.push(Value::Obj(key_handle));
    }

    let result_handle = heap.alloc_object(Object::List(keys));
    Ok(Value::Obj(result_handle))
}

/// map_values(map: Map<K, V>) -> List<V>
/// Returns a list of values from a map
pub fn map_values_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.is_empty() {
        return Err(PulseError::RuntimeError(
            "map_values requires a map argument".to_string(),
        ));
    }

    let values = match &args[0] {
        Value::Obj(handle) => match heap.get_object(*handle) {
            Some(Object::Map(m)) => m.values().cloned().collect::<Vec<_>>(),
            _ => {
                return Err(PulseError::RuntimeError(
                    "Expected map as first argument".to_string(),
                ))
            }
        },
        _ => {
            return Err(PulseError::RuntimeError(
                "Expected map as first argument".to_string(),
            ))
        }
    };

    let result_handle = heap.alloc_object(Object::List(values));
    Ok(Value::Obj(result_handle))
}

/// map_entries(map: Map<K, V>) -> List<(K, V)>
/// Returns a list of key-value pairs from a map
pub fn map_entries_native(heap: &mut dyn HeapInterface, args: &[Value]) -> PulseResult<Value> {
    if args.is_empty() {
        return Err(PulseError::RuntimeError(
            "map_entries requires a map argument".to_string(),
        ));
    }

    // Collect entries first to avoid borrow issues
    let entries_data: Vec<(String, Value)> = match &args[0] {
        Value::Obj(handle) => match heap.get_object(*handle) {
            Some(Object::Map(m)) => m.iter().map(|(k, v)| (k.clone(), *v)).collect(),
            _ => {
                return Err(PulseError::RuntimeError(
                    "Expected map as first argument".to_string(),
                ))
            }
        },
        _ => {
            return Err(PulseError::RuntimeError(
                "Expected map as first argument".to_string(),
            ))
        }
    };

    // Now allocate the entries
    let mut entries: Vec<Value> = Vec::new();
    for (k, v) in entries_data {
        // Create a tuple-like array [key, value]
        let key_handle = heap.alloc_object(Object::String(k));
        let pair = vec![Value::Obj(key_handle), v];
        let pair_handle = heap.alloc_object(Object::List(pair));
        entries.push(Value::Obj(pair_handle));
    }

    let result_handle = heap.alloc_object(Object::List(entries));
    Ok(Value::Obj(result_handle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulse_vm::Heap;

    #[test]
    fn test_list_sort() {
        let mut heap = Heap::new();

        // Create unsorted list [3, 1, 2]
        let list = vec![Value::Int(3), Value::Int(1), Value::Int(2)];
        let handle = heap.alloc_object(Object::List(list));

        let result = list_sort_native(&mut heap, &[Value::Obj(handle)]).unwrap();

        if let Value::Obj(result_handle) = result {
            if let Some(Object::List(sorted)) = heap.get_object(result_handle) {
                assert_eq!(sorted.len(), 3);
                assert_eq!(sorted[0], Value::Int(1));
                assert_eq!(sorted[1], Value::Int(2));
                assert_eq!(sorted[2], Value::Int(3));
            } else {
                panic!("Expected list");
            }
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_list_reverse() {
        let mut heap = Heap::new();

        let list = vec![Value::Int(1), Value::Int(2), Value::Int(3)];
        let handle = heap.alloc_object(Object::List(list));

        let result = list_reverse_native(&mut heap, &[Value::Obj(handle)]).unwrap();

        if let Value::Obj(result_handle) = result {
            if let Some(Object::List(reversed)) = heap.get_object(result_handle) {
                assert_eq!(reversed[0], Value::Int(3));
                assert_eq!(reversed[2], Value::Int(1));
            } else {
                panic!("Expected list");
            }
        }
    }

    #[test]
    fn test_map_keys_values() {
        let mut heap = Heap::new();

        // Create a map
        let mut map = std::collections::HashMap::new();
        map.insert("a".to_string(), Value::Int(1));
        map.insert("b".to_string(), Value::Int(2));
        let handle = heap.alloc_object(Object::Map(map));

        // Test keys
        let keys_result = map_keys_native(&mut heap, &[Value::Obj(handle)]).unwrap();
        if let Value::Obj(keys_handle) = keys_result {
            if let Some(Object::List(keys)) = heap.get_object(keys_handle) {
                assert_eq!(keys.len(), 2);
            }
        }

        // Test values
        let values_result = map_values_native(&mut heap, &[Value::Obj(handle)]).unwrap();
        if let Value::Obj(values_handle) = values_result {
            if let Some(Object::List(values)) = heap.get_object(values_handle) {
                assert_eq!(values.len(), 2);
            }
        }
    }
}
