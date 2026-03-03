//! File I/O native functions
//!
//! Provides file reading, writing, and manipulation operations.

use futures::FutureExt;
use pulse_ast::object::{HeapInterface, Object};
use pulse_ast::{PulseError, PulseResult, Value};
use std::future::Future;
use std::pin::Pin;
use tokio::fs;

/// read_file(path: String) -> String
/// Reads entire file contents as a string
pub fn read_file_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    async move {
        if args.is_empty() {
            return Err(PulseError::RuntimeError(
                "read_file requires a path argument".to_string(),
            ));
        }

        let path = match &args[0] {
            Value::Obj(handle) => match heap.get_object(*handle) {
                Some(Object::String(s)) => s.clone(),
                _ => return Err(PulseError::RuntimeError("Expected string path".to_string())),
            },
            _ => return Err(PulseError::RuntimeError("Expected string path".to_string())),
        };

        match fs::read_to_string(&path).await {
            Ok(content) => {
                let handle = heap.alloc_object(Object::String(content));
                Ok(Value::Obj(handle))
            }
            Err(e) => Err(PulseError::RuntimeError(format!(
                "Failed to read file '{}': {}",
                path, e
            ))),
        }
    }
    .boxed()
}

/// write_file(path: String, content: String) -> Unit
/// Writes string content to a file (overwrites existing)
pub fn write_file_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    async move {
        if args.len() < 2 {
            return Err(PulseError::RuntimeError(
                "write_file requires path and content arguments".to_string(),
            ));
        }

        let path = match &args[0] {
            Value::Obj(handle) => match heap.get_object(*handle) {
                Some(Object::String(s)) => s.clone(),
                _ => return Err(PulseError::RuntimeError("Expected string path".to_string())),
            },
            _ => return Err(PulseError::RuntimeError("Expected string path".to_string())),
        };

        let content = match &args[1] {
            Value::Obj(handle) => match heap.get_object(*handle) {
                Some(Object::String(s)) => s.clone(),
                _ => {
                    return Err(PulseError::RuntimeError(
                        "Expected string content".to_string(),
                    ))
                }
            },
            _ => {
                return Err(PulseError::RuntimeError(
                    "Expected string content".to_string(),
                ))
            }
        };

        match fs::write(&path, content).await {
            Ok(_) => Ok(Value::Unit),
            Err(e) => Err(PulseError::RuntimeError(format!(
                "Failed to write file '{}': {}",
                path, e
            ))),
        }
    }
    .boxed()
}

/// append_file(path: String, content: String) -> Unit
/// Appends string content to a file
pub fn append_file_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    async move {
        if args.len() < 2 {
            return Err(PulseError::RuntimeError(
                "append_file requires path and content arguments".to_string(),
            ));
        }

        let path = match &args[0] {
            Value::Obj(handle) => match heap.get_object(*handle) {
                Some(Object::String(s)) => s.clone(),
                _ => return Err(PulseError::RuntimeError("Expected string path".to_string())),
            },
            _ => return Err(PulseError::RuntimeError("Expected string path".to_string())),
        };

        let content = match &args[1] {
            Value::Obj(handle) => match heap.get_object(*handle) {
                Some(Object::String(s)) => s.clone(),
                _ => {
                    return Err(PulseError::RuntimeError(
                        "Expected string content".to_string(),
                    ))
                }
            },
            _ => {
                return Err(PulseError::RuntimeError(
                    "Expected string content".to_string(),
                ))
            }
        };

        use tokio::io::AsyncWriteExt;
        match fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .await
        {
            Ok(mut file) => match file.write_all(content.as_bytes()).await {
                Ok(_) => Ok(Value::Unit),
                Err(e) => Err(PulseError::RuntimeError(format!(
                    "Failed to append to file '{}': {}",
                    path, e
                ))),
            },
            Err(e) => Err(PulseError::RuntimeError(format!(
                "Failed to open file '{}': {}",
                path, e
            ))),
        }
    }
    .boxed()
}

/// read_lines(path: String) -> List<String>
/// Reads file and returns list of lines
pub fn read_lines_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    async move {
        if args.is_empty() {
            return Err(PulseError::RuntimeError(
                "read_lines requires a path argument".to_string(),
            ));
        }

        let path = match &args[0] {
            Value::Obj(handle) => match heap.get_object(*handle) {
                Some(Object::String(s)) => s.clone(),
                _ => return Err(PulseError::RuntimeError("Expected string path".to_string())),
            },
            _ => return Err(PulseError::RuntimeError("Expected string path".to_string())),
        };

        match fs::read_to_string(&path).await {
            Ok(content) => {
                let lines: Vec<Value> = content
                    .lines()
                    .map(|line| {
                        let handle = heap.alloc_object(Object::String(line.to_string()));
                        Value::Obj(handle)
                    })
                    .collect();
                let list_handle = heap.alloc_object(Object::List(lines));
                Ok(Value::Obj(list_handle))
            }
            Err(e) => Err(PulseError::RuntimeError(format!(
                "Failed to read file '{}': {}",
                path, e
            ))),
        }
    }
    .boxed()
}

/// file_exists(path: String) -> Bool
/// Checks if a file exists
pub fn file_exists_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    async move {
        if args.is_empty() {
            return Err(PulseError::RuntimeError(
                "file_exists requires a path argument".to_string(),
            ));
        }

        let path = match &args[0] {
            Value::Obj(handle) => match heap.get_object(*handle) {
                Some(Object::String(s)) => s.clone(),
                _ => return Err(PulseError::RuntimeError("Expected string path".to_string())),
            },
            _ => return Err(PulseError::RuntimeError("Expected string path".to_string())),
        };

        let exists = tokio::fs::metadata(&path).await.is_ok();
        Ok(Value::Bool(exists))
    }
    .boxed()
}

/// create_dir(path: String) -> Unit
/// Creates a directory
pub fn create_dir_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    async move {
        if args.is_empty() {
            return Err(PulseError::RuntimeError(
                "create_dir requires a path argument".to_string(),
            ));
        }

        let path = match &args[0] {
            Value::Obj(handle) => match heap.get_object(*handle) {
                Some(Object::String(s)) => s.clone(),
                _ => return Err(PulseError::RuntimeError("Expected string path".to_string())),
            },
            _ => return Err(PulseError::RuntimeError("Expected string path".to_string())),
        };

        match fs::create_dir_all(&path).await {
            Ok(_) => Ok(Value::Unit),
            Err(e) => Err(PulseError::RuntimeError(format!(
                "Failed to create directory '{}': {}",
                path, e
            ))),
        }
    }
    .boxed()
}

/// remove_file(path: String) -> Unit
/// Removes a file
pub fn remove_file_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    async move {
        if args.is_empty() {
            return Err(PulseError::RuntimeError(
                "remove_file requires a path argument".to_string(),
            ));
        }

        let path = match &args[0] {
            Value::Obj(handle) => match heap.get_object(*handle) {
                Some(Object::String(s)) => s.clone(),
                _ => return Err(PulseError::RuntimeError("Expected string path".to_string())),
            },
            _ => return Err(PulseError::RuntimeError("Expected string path".to_string())),
        };

        match fs::remove_file(&path).await {
            Ok(_) => Ok(Value::Unit),
            Err(e) => Err(PulseError::RuntimeError(format!(
                "Failed to remove file '{}': {}",
                path, e
            ))),
        }
    }
    .boxed()
}

/// delete_file(path: String) -> Unit
/// Alias for remove_file
pub fn delete_file_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    remove_file_native(heap, args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulse_vm::Heap;

    #[tokio::test]
    async fn test_write_and_read_file() {
        let mut heap = Heap::new();
        let test_path = "test_io_file.txt";
        let test_content = "Hello, Pulse!";

        // Write file
        let path_handle = heap.alloc_object(Object::String(test_path.to_string()));
        let content_handle = heap.alloc_object(Object::String(test_content.to_string()));

        let write_args = vec![Value::Obj(path_handle), Value::Obj(content_handle)];
        let result = write_file_native(&mut heap, &write_args).await;
        assert!(result.is_ok());

        // Read file
        let read_args = vec![Value::Obj(path_handle)];
        let result = read_file_native(&mut heap, &read_args).await;
        assert!(result.is_ok());

        // Verify content
        if let Ok(Value::Obj(handle)) = result {
            if let Some(Object::String(content)) = heap.get_object(handle) {
                assert_eq!(content, test_content);
            } else {
                panic!("Expected string object");
            }
        } else {
            panic!("Expected object value");
        }

        // Cleanup
        let _ = fs::remove_file(test_path).await;
    }

    #[tokio::test]
    async fn test_append_file() {
        let mut heap = Heap::new();
        let test_path = "test_append_file.txt";

        // Write initial content
        let path_handle = heap.alloc_object(Object::String(test_path.to_string()));
        let content1 = heap.alloc_object(Object::String("Hello".to_string()));

        let _ =
            write_file_native(&mut heap, &[Value::Obj(path_handle), Value::Obj(content1)]).await;

        // Append more content
        let content2 = heap.alloc_object(Object::String(" World".to_string()));
        let _ =
            append_file_native(&mut heap, &[Value::Obj(path_handle), Value::Obj(content2)]).await;

        // Read and verify
        let result = read_file_native(&mut heap, &[Value::Obj(path_handle)]).await;
        if let Ok(Value::Obj(handle)) = result {
            if let Some(Object::String(content)) = heap.get_object(handle) {
                assert_eq!(content, "Hello World");
            }
        }

        // Cleanup
        let _ = fs::remove_file(test_path).await;
    }

    #[tokio::test]
    async fn test_create_and_remove_dir() {
        let mut heap = Heap::new();
        let test_dir = "test_io_dir";

        // Create directory
        let path_handle = heap.alloc_object(Object::String(test_dir.to_string()));
        let result = create_dir_native(&mut heap, &[Value::Obj(path_handle)]).await;
        assert!(result.is_ok());

        // Verify it exists
        assert!(fs::metadata(test_dir).await.is_ok());

        // Cleanup
        let _ = fs::remove_dir(test_dir).await;
    }

    #[tokio::test]
    async fn test_read_lines() {
        let mut heap = Heap::new();
        let test_path = "test_lines.txt";
        let test_content = "Line 1\nLine 2\nLine 3";

        // Write test file
        let path_handle = heap.alloc_object(Object::String(test_path.to_string()));
        let content_handle = heap.alloc_object(Object::String(test_content.to_string()));
        let _ = write_file_native(
            &mut heap,
            &[Value::Obj(path_handle), Value::Obj(content_handle)],
        )
        .await;

        // Read lines
        let result = read_lines_native(&mut heap, &[Value::Obj(path_handle)]).await;
        assert!(result.is_ok());

        if let Ok(Value::Obj(handle)) = result {
            if let Some(Object::List(lines)) = heap.get_object(handle) {
                assert_eq!(lines.len(), 3);
            } else {
                panic!("Expected list object");
            }
        }

        // Cleanup
        let _ = fs::remove_file(test_path).await;
    }
}
