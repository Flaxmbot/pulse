//! FileSystem native functions

use futures::FutureExt;
use pulse_ast::object::{HeapInterface, Object};
use pulse_ast::{PulseError, PulseResult, Value};
use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use tokio::fs;

/// Maximum allowed path length
const MAX_PATH_LENGTH: usize = 4096;

/// Sanitizes a file path to prevent path traversal attacks
/// Returns Ok(sanitized_path) or Err if path traversal is detected
pub fn sanitize_path(path: &str) -> PulseResult<PathBuf> {
    // Check for null bytes
    if path.contains('\0') {
        return Err(PulseError::PathTraversalAttempted(
            "Path contains null bytes".into(),
        ));
    }

    // Check path length
    if path.len() > MAX_PATH_LENGTH {
        return Err(PulseError::PathTraversalAttempted(
            "Path exceeds maximum length".into(),
        ));
    }

    // Check for path traversal sequences
    if path.contains("..") || path.starts_with('~') || path.starts_with('/') {
        return Err(PulseError::PathTraversalAttempted(path.to_string()));
    }

    // Get the current working directory as base
    let current_dir = std::env::current_dir()
        .map_err(|e| PulseError::IoError(format!("Failed to get current directory: {}", e)))?;

    // Parse the path
    let path_buf = PathBuf::from(path);

    // If path is absolute, reject it
    if path_buf.is_absolute() {
        return Err(PulseError::PathTraversalAttempted(
            "Absolute paths not allowed".into(),
        ));
    }

    // Normalize: resolve . and remove duplicate slashes
    let normalized = path.replace("/./", "/").replace("//", "/");

    // Re-check for traversal after normalization
    if normalized.contains("..") {
        return Err(PulseError::PathTraversalAttempted(normalized));
    }

    // Join with current directory to get the full path
    let full_path = current_dir.join(&normalized);

    // Canonicalize to resolve any symlinks and verify it's within project directory
    match full_path.canonicalize() {
        Ok(canonical) => {
            // Ensure the resolved path is within the project directory
            if !canonical.starts_with(&current_dir) {
                return Err(PulseError::PathTraversalAttempted(format!(
                    "Path escapes project directory: {}",
                    path
                )));
            }
            Ok(canonical)
        }
        Err(_) => {
            // If canonicalize fails (e.g., file doesn't exist), still check it's within bounds
            if !full_path.starts_with(&current_dir) {
                return Err(PulseError::PathTraversalAttempted(format!(
                    "Path escapes project directory: {}",
                    path
                )));
            }
            Ok(full_path)
        }
    }
}

/// Validates a path is within the project directory without canonicalizing
#[allow(dead_code)]
fn validate_project_path(path: &str) -> PulseResult<PathBuf> {
    let sanitized = sanitize_path(path)?;
    Ok(sanitized)
}

/// read_dir(path: String) -> List
/// Reads a directory and returns a list of entries (files and subdirectories)
pub fn read_dir_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 1 {
            return Err(PulseError::RuntimeError(
                "read_dir expects 1 argument".into(),
            ));
        }

        let path_str = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into(),
                    });
                }
            }
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: args[0].type_name(),
                })
            }
        };

        // Sanitize path for security
        let path = sanitize_path(&path_str)?;
        let mut entries = Vec::new();

        let mut dir = fs::read_dir(path)
            .await
            .map_err(|e| PulseError::RuntimeError(format!("Failed to read directory: {}", e)))?;

        while let Some(entry) = dir
            .next_entry()
            .await
            .map_err(|e| PulseError::RuntimeError(format!("Failed to read entry: {}", e)))?
        {
            let entry_name = entry.file_name().to_string_lossy().to_string();
            let entry_path = entry.path();
            let is_dir = entry_path.is_dir();

            let mut map: HashMap<String, Value> = HashMap::new();
            map.insert(
                "name".to_string(),
                Value::Obj(heap.alloc_object(Object::String(entry_name))),
            );
            map.insert("is_directory".to_string(), Value::Bool(is_dir));
            map.insert(
                "path".to_string(),
                Value::Obj(
                    heap.alloc_object(Object::String(entry_path.to_string_lossy().to_string())),
                ),
            );

            entries.push(Value::Obj(heap.alloc_object(Object::Map(map))));
        }

        let list_handle = heap.alloc_object(Object::List(entries));
        Ok(Value::Obj(list_handle))
    }
    .boxed()
}

/// create_dir(path: String) -> Bool
/// Creates a directory (and parent directories if they don't exist)
pub fn create_dir_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 1 {
            return Err(PulseError::RuntimeError(
                "create_dir expects 1 argument".into(),
            ));
        }

        let path_str = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into(),
                    });
                }
            }
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: args[0].type_name(),
                })
            }
        };

        // Sanitize path for security
        let path = sanitize_path(&path_str)?;
        fs::create_dir_all(path)
            .await
            .map_err(|e| PulseError::RuntimeError(format!("Failed to create directory: {}", e)))?;

        Ok(Value::Bool(true))
    }
    .boxed()
}

/// remove_dir(path: String) -> Bool
/// Removes an empty directory
pub fn remove_dir_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 1 {
            return Err(PulseError::RuntimeError(
                "remove_dir expects 1 argument".into(),
            ));
        }

        let path_str = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into(),
                    });
                }
            }
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: args[0].type_name(),
                })
            }
        };

        // Sanitize path for security
        let path = sanitize_path(&path_str)?;
        fs::remove_dir(path)
            .await
            .map_err(|e| PulseError::RuntimeError(format!("Failed to remove directory: {}", e)))?;

        Ok(Value::Bool(true))
    }
    .boxed()
}

/// remove_file(path: String) -> Bool
/// Removes a file
pub fn remove_file_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 1 {
            return Err(PulseError::RuntimeError(
                "remove_file expects 1 argument".into(),
            ));
        }

        let path_str = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into(),
                    });
                }
            }
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: args[0].type_name(),
                })
            }
        };

        // Sanitize path for security
        let path = sanitize_path(&path_str)?;
        fs::remove_file(path)
            .await
            .map_err(|e| PulseError::RuntimeError(format!("Failed to remove file: {}", e)))?;

        Ok(Value::Bool(true))
    }
    .boxed()
}

/// file_exists(path: String) -> Bool
/// Checks if a file or directory exists
pub fn file_exists_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 1 {
            return Err(PulseError::RuntimeError(
                "file_exists expects 1 argument".into(),
            ));
        }

        let path_str = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into(),
                    });
                }
            }
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: args[0].type_name(),
                })
            }
        };

        // Sanitize path for security
        let path = sanitize_path(&path_str)?;
        let exists = path.exists();

        Ok(Value::Bool(exists))
    }
    .boxed()
}

/// get_metadata(path: String) -> Map
/// Gets file/directory metadata (size, created, modified, is_file, is_dir)
pub fn get_metadata_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 1 {
            return Err(PulseError::RuntimeError(
                "get_metadata expects 1 argument".into(),
            ));
        }

        let path_str = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into(),
                    });
                }
            }
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: args[0].type_name(),
                })
            }
        };

        // Sanitize path for security
        let path = sanitize_path(&path_str)?;
        let metadata = fs::metadata(path)
            .await
            .map_err(|e| PulseError::RuntimeError(format!("Failed to get metadata: {}", e)))?;

        let mut map: HashMap<String, Value> = HashMap::new();
        map.insert("size".to_string(), Value::Int(metadata.len() as i64));
        map.insert("is_file".to_string(), Value::Bool(metadata.is_file()));
        map.insert("is_directory".to_string(), Value::Bool(metadata.is_dir()));

        if let Ok(modified) = metadata.modified() {
            let duration = modified
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| PulseError::RuntimeError(format!("Time error: {}", e)))?;
            map.insert("modified".to_string(), Value::Float(duration.as_secs_f64()));
        }

        if let Ok(created) = metadata.created() {
            let duration = created
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| PulseError::RuntimeError(format!("Time error: {}", e)))?;
            map.insert("created".to_string(), Value::Float(duration.as_secs_f64()));
        }

        let map_handle = heap.alloc_object(Object::Map(map));
        Ok(Value::Obj(map_handle))
    }
    .boxed()
}

/// copy_file(source: String, dest: String) -> Bool
/// Copies a file from source to destination
pub fn copy_file_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 2 {
            return Err(PulseError::RuntimeError(
                "copy_file expects 2 arguments".into(),
            ));
        }

        let source = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into(),
                    });
                }
            }
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: args[0].type_name(),
                })
            }
        };

        let dest = match &args[1] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into(),
                    });
                }
            }
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: args[1].type_name(),
                })
            }
        };

        // Sanitize paths for security
        let source = sanitize_path(&source)?;
        let dest = sanitize_path(&dest)?;
        fs::copy(&source, &dest)
            .await
            .map_err(|e| PulseError::RuntimeError(format!("Failed to copy file: {}", e)))?;

        Ok(Value::Bool(true))
    }
    .boxed()
}

/// rename_file(old_path: String, new_path: String) -> Bool
/// Renames/moves a file or directory
pub fn rename_file_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 2 {
            return Err(PulseError::RuntimeError(
                "rename_file expects 2 arguments".into(),
            ));
        }

        let old_path = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into(),
                    });
                }
            }
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: args[0].type_name(),
                })
            }
        };

        let new_path = match &args[1] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into(),
                    });
                }
            }
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: args[1].type_name(),
                })
            }
        };

        // Sanitize paths for security
        let old_path = sanitize_path(&old_path)?;
        let new_path = sanitize_path(&new_path)?;
        fs::rename(&old_path, &new_path)
            .await
            .map_err(|e| PulseError::RuntimeError(format!("Failed to rename: {}", e)))?;

        Ok(Value::Bool(true))
    }
    .boxed()
}

/// list_dir(path: String) -> List
/// Lists all files and directories in a path (simpler version)
pub fn list_dir_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 1 {
            return Err(PulseError::RuntimeError(
                "list_dir expects 1 argument".into(),
            ));
        }

        let path_str = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into(),
                    });
                }
            }
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: args[0].type_name(),
                })
            }
        };

        // Sanitize path for security
        let path = sanitize_path(&path_str)?;
        let mut entries = Vec::new();

        let mut dir = fs::read_dir(path)
            .await
            .map_err(|e| PulseError::RuntimeError(format!("Failed to read directory: {}", e)))?;

        while let Some(entry) = dir
            .next_entry()
            .await
            .map_err(|e| PulseError::RuntimeError(format!("Failed to read entry: {}", e)))?
        {
            let entry_name = entry.file_name().to_string_lossy().to_string();
            let handle = heap.alloc_object(Object::String(entry_name));
            entries.push(Value::Obj(handle));
        }

        let list_handle = heap.alloc_object(Object::List(entries));
        Ok(Value::Obj(list_handle))
    }
    .boxed()
}

/// is_file(path: String) -> Bool
/// Checks if path is a file
pub fn is_file_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 1 {
            return Err(PulseError::RuntimeError(
                "is_file expects 1 argument".into(),
            ));
        }

        let path_str = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into(),
                    });
                }
            }
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: args[0].type_name(),
                })
            }
        };

        // Sanitize path for security
        let path = sanitize_path(&path_str)?;
        let is_file = path.is_file();

        Ok(Value::Bool(is_file))
    }
    .boxed()
}

/// is_dir(path: String) -> Bool
/// Checks if path is a directory
pub fn is_dir_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 1 {
            return Err(PulseError::RuntimeError("is_dir expects 1 argument".into()));
        }

        let path_str = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into(),
                    });
                }
            }
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: args[0].type_name(),
                })
            }
        };

        // Sanitize path for security
        let path = sanitize_path(&path_str)?;
        let is_dir = path.is_dir();

        Ok(Value::Bool(is_dir))
    }
    .boxed()
}

/// read_bytes(path: String) -> List
/// Reads a file as a list of bytes
pub fn read_bytes_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 1 {
            return Err(PulseError::RuntimeError(
                "read_bytes expects 1 argument".into(),
            ));
        }

        let path_str = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into(),
                    });
                }
            }
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: args[0].type_name(),
                })
            }
        };

        // Sanitize path for security
        let path = sanitize_path(&path_str)?;
        let bytes = fs::read(&path)
            .await
            .map_err(|e| PulseError::RuntimeError(format!("Failed to read file: {}", e)))?;

        let list: Vec<Value> = bytes.into_iter().map(|b| Value::Int(b as i64)).collect();

        let list_handle = heap.alloc_object(Object::List(list));
        Ok(Value::Obj(list_handle))
    }
    .boxed()
}

/// write_bytes(path: String, data: List) -> Bool
/// Writes a list of bytes to a file
pub fn write_bytes_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 2 {
            return Err(PulseError::RuntimeError(
                "write_bytes expects 2 arguments".into(),
            ));
        }

        let path_str = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into(),
                    });
                }
            }
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: args[0].type_name(),
                })
            }
        };

        let data = match &args[1] {
            Value::Obj(h) => {
                if let Some(Object::List(list)) = heap.get_object(*h) {
                    list.iter()
                        .map(|v| match v {
                            Value::Int(i) => Ok(*i as u8),
                            _ => Err(PulseError::TypeMismatch {
                                expected: "int".into(),
                                got: v.type_name(),
                            }),
                        })
                        .collect::<Result<Vec<u8>, _>>()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "list".into(),
                        got: "object".into(),
                    });
                }
            }
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "list".into(),
                    got: args[1].type_name(),
                })
            }
        }?;

        // Sanitize path for security
        let path = sanitize_path(&path_str)?;
        fs::write(&path, data)
            .await
            .map_err(|e| PulseError::RuntimeError(format!("Failed to write file: {}", e)))?;

        Ok(Value::Bool(true))
    }
    .boxed()
}

/// get_current_dir() -> String
/// Gets the current working directory
pub fn get_current_dir_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if !args.is_empty() {
            return Err(PulseError::RuntimeError(
                "get_current_dir expects 0 arguments".into(),
            ));
        }

        let cwd = std::env::current_dir()
            .map_err(|e| PulseError::RuntimeError(format!("Failed to get current dir: {}", e)))?;

        let cwd_str = cwd.to_string_lossy().to_string();
        let handle = heap.alloc_object(Object::String(cwd_str));
        Ok(Value::Obj(handle))
    }
    .boxed()
}

/// set_current_dir(path: String) -> Bool
/// Sets the current working directory
pub fn set_current_dir_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 1 {
            return Err(PulseError::RuntimeError(
                "set_current_dir expects 1 argument".into(),
            ));
        }

        let path_str = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into(),
                    });
                }
            }
            _ => {
                return Err(PulseError::TypeMismatch {
                    expected: "string".into(),
                    got: args[0].type_name(),
                })
            }
        };

        // Sanitize path for security (but don't require it to exist)
        let current_dir = std::env::current_dir()
            .map_err(|e| PulseError::IoError(format!("Failed to get current directory: {}", e)))?;
        let new_path = current_dir.join(&path_str);

        // Verify the path doesn't escape project directory
        if !new_path.starts_with(&current_dir) {
            return Err(PulseError::PathTraversalAttempted(format!(
                "Path escapes project directory: {}",
                path_str
            )));
        }

        std::env::set_current_dir(&new_path)
            .map_err(|e| PulseError::RuntimeError(format!("Failed to set current dir: {}", e)))?;

        Ok(Value::Bool(true))
    }
    .boxed()
}
