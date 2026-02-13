//! File I/O native functions

use pulse_core::{Value, PulseResult, PulseError};
use pulse_core::object::{HeapInterface, Object};
use tokio::fs;
use std::pin::Pin;
use std::future::Future;
use futures::FutureExt;

/// read_file(path: String) -> String
/// Reads entire file contents as a string
pub fn read_file_native<'a>(heap: &'a mut dyn HeapInterface, args: &'a [Value]) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec(); // Clone args to move into future? Or handle differently.
    // Value is Clone, so args.to_vec() is fine.
    // However, heap access inside future?
    // SyncNativeFn takes `&mut dyn HeapInterface`.
    // AsyncNativeFn takes `&mut dyn HeapInterface` too?
    // Wait, let's check `AsyncNativeFn` definition in `pulse_core`.
    // It likely is `fn(...) -> Pin<Box<dyn Future...>>`.
    // Inside the future, we can't use `heap` if it's a reference with lifetime 'a, unless we use it before await or capture it?
    // `HeapInterface` is not Send? VM is Send (or was made Send).
    // `Heap` struct methods are synchronous.
    // If we need to allocate the result string on heap, we need access to heap *after* await.
    // This is tricky with `&mut HeapInterface`.
    // We can't hold `&mut HeapInterface` across await.
    //
    // SOLUTION:
    // 1. Read arguments (extract path) synchronously.
    // 2. Perform async I/O (read file).
    // 3. Return the content string.
    // 4. BUT `AsyncNativeFn` returns `PulseResult<Value>`. We need to allocate `Object::String` on heap.
    //    We can't access heap after await if we don't have it.
    //
    // Alternative: Return `String` from async part, and have a wrapper? No, native fn must return Value.
    //
    // Maybe we pass `Arc<Mutex<Heap>>`? No, VM structure is single threaded usually but running in Actor.
    // Use `vm.heap` which is available.
    //
    // If we can't access heap after await, we have a problem for creating Objects.
    //
    // Check `tcp_connect_native` implementation in `networking.rs`.
    // It returns `Value::Unit` or `Value::Obj` (Socket).
    // How does it allocate `Socket`?
    // It captures `heap`?
    // Let's check `networking.rs` again.
    //
    // `tcp_connect_native` signature: `fn tcp_connect_native<'a>(_heap: &'a mut dyn HeapInterface, args: &'a [Value]) -> Pin<Box<dyn Future... + 'a>>`
    // It likely doesn't use `_heap` after await?
    // `tcp_connect` returns `Value::Unit` (or `Socket`?).
    // `networking.rs` shows:
    // `match TcpStream::connect(addr).await { Ok(stream) => { ... Value::Obj(heap.alloc...) } }`
    // WAIT. If `heap` is captured in `async move`, and `heap` is `&'a mut ...`.
    // `&mut T` is not `Copy`. We cannot move it into `async block`.
    // 
    // This suggests my `networking.rs` implementation might be wrong or compiled because I didn't verify it properly?
    // Or I used `Arc`?
    //
    // Let's look at `networking.rs` implementation I wrote in `Previous Session`.
    // `tcp_connect_native` had `_heap`.
    // Ah, I might have used `_heap` (unused) or used it.
    // If I use it, it must be `Send`?
    // `dyn HeapInterface + 'a` is (?) `Send`.
    // But `&mut` reference cannot be moved if it's borrowed?
    //
    // Actually, `add_native_async` signature: `for<'a> fn(&'a mut ..., &'a ...) -> Pin<Box<dyn Future... + 'a>>`.
    // The Future captures the lifetime 'a.
    // So inside the future, we HAVE the reference `heap`.
    // BUT `&mut Heap` is not `Send` unless `Heap` is `Send`. `Heap` contains `Rc`? No, `Vec<Object>`. `Object` contains `Arc`. So `Heap` is `Send`.
    // So `&mut Heap` is `Send`.
    // BUT we cannot hold `&mut Heap` across an `.await` point if the future itself is executed by a runtime interacting with that heap?
    // No, the *Actor* runs the future. The Actor *owns* the VM/Heap.
    // While the future is polling, the Actor is blocked on it (or yielding).
    // So it should be safe to hold the reference?
    //
    // However, Rust borrow checker might complain if we hold `&mut heap` across yield point?
    //
    // Let's try to implement `read_file_native` as:
    // Extract path (needs heap?) - `args[0]` might be object handle. `heap.get_object` needed.
    // Do this SYNC part first.
    // Then `async move { ... fs::read ... }`.
    // Then `heap.alloc`?
    // If `async move` block captures `heap`, and we `await fs::read`, `heap` is held.
    //
    // Let's write the code.
    
    // First, extract path synchronously.
    // We can't access `args` inside async block easily if they are references.
    // So we clone args or extract what we need.
    
    let path_str = match &args[0] {
        Value::Obj(h) => {
             // We need access to heap to get string.
             // We can use `heap` here.
             // But we also need `heap` INSIDE the future to alloc the result.
             // We cannot move `heap` into future AND use it here?
             // `heap` is `&mut`. We can reborrow?
             // But valid lifetime...
             //
             // We can split the logic?
             // But `read_file_native` is one function.
             //
             // Maybe `heap` shouldn't be captured?
             // If we return `String` from file read, can we return `Result<String>` and let caller alloc?
             // No, `NativeFn` signature is fixed.
             //
             // Strategy:
             // 1. Extract path using `heap` (methods on `heap` take `&self` or `&mut self`).
             // 2. Clone the string path.
             // 3. Perform async read.
             // 4. Alloc result using `heap`.
             //
             // Issue: Capture `heap` in async block.
             // `path` is owned String.
             //
             // `async move { let content = fs::read_to_string(&path).await?; heap.alloc... }`
             // This captures `heap`.
             // If `heap` is `&mut dyn HeapInterface`, it is `Send` (if dyn HeapInterface expects Send? Yes, `VM` is Send).
             //
             // Note: `args` access in async block? `args` is `&[Value]`.
             // If we extract path before, we don't need `args`.
             
             if let Some(Object::String(s)) = heap.get_object(*h) {
                 s.clone()
             } else {
                 return async move { Err(PulseError::TypeMismatch { expected: "string".into(), got: "object".into() }) }.boxed();
             }
        }
        _ => return async move { Err(PulseError::TypeMismatch { expected: "string".into(), got: args[0].type_name() }) }.boxed(),
    };

    async move {
        match fs::read_to_string(&path_str).await {
            Ok(content) => {
                let handle = heap.alloc_object(Object::String(content));
                Ok(Value::Obj(handle))
            }
            Err(e) => Err(PulseError::RuntimeError(format!("Failed to read file '{}': {}", path_str, e))),
        }
    }.boxed()
}

/// write_file(path: String, content: String) -> Bool
/// Writes content to file, returns true on success
pub fn write_file_native<'a>(heap: &'a mut dyn HeapInterface, args: &'a [Value]) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    // Extract arguments
    let path_str = if args.len() >= 1 {
         match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return async move { Err(PulseError::TypeMismatch { expected: "string".into(), got: "object".into() }) }.boxed();
                }
            }
            _ => return async move { Err(PulseError::TypeMismatch { expected: "string".into(), got: args[0].type_name() }) }.boxed(),
         }
    } else {
        return async move { Err(PulseError::RuntimeError("write_file expects 2 arguments".into())) }.boxed();
    };

    let content_str = if args.len() >= 2 {
         match &args[1] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return async move { Err(PulseError::TypeMismatch { expected: "string".into(), got: "object".into() }) }.boxed();
                }
            }
            _ => return async move { Err(PulseError::TypeMismatch { expected: "string".into(), got: args[1].type_name() }) }.boxed(),
         }
    } else {
        return async move { Err(PulseError::RuntimeError("write_file expects 2 arguments".into())) }.boxed();
    };


    async move {
        match fs::write(&path_str, &content_str).await {
            Ok(_) => Ok(Value::Bool(true)),
            Err(_) => Ok(Value::Bool(false)),
        }
    }.boxed()
}

/// file_exists(path: String) -> Bool
/// Checks if a file exists
pub fn file_exists_native<'a>(heap: &'a mut dyn HeapInterface, args: &'a [Value]) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let path_str = if args.len() >= 1 {
         match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return async move { Err(PulseError::TypeMismatch { expected: "string".into(), got: "object".into() }) }.boxed();
                }
            }
            _ => return async move { Err(PulseError::TypeMismatch { expected: "string".into(), got: args[0].type_name() }) }.boxed(),
         }
    } else {
        return async move { Err(PulseError::RuntimeError("file_exists expects 1 argument".into())) }.boxed();
    };

    async move {
        // file_exists is sync in std::path usually, but tokio usually has fs::try_exists
        // tokio::fs::try_exists is async.
        match fs::try_exists(&path_str).await {
            Ok(exists) => Ok(Value::Bool(exists)),
            Err(_) => Ok(Value::Bool(false)), // mimic std::path::exists behavior? Or return error?
        }
    }.boxed()
}

/// delete_file(path: String) -> Bool
/// Deletes a file, returns true on success
pub fn delete_file_native<'a>(heap: &'a mut dyn HeapInterface, args: &'a [Value]) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let path_str = if args.len() >= 1 {
         match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return async move { Err(PulseError::TypeMismatch { expected: "string".into(), got: "object".into() }) }.boxed();
                }
            }
            _ => return async move { Err(PulseError::TypeMismatch { expected: "string".into(), got: args[0].type_name() }) }.boxed(),
         }
    } else {
        return async move { Err(PulseError::RuntimeError("delete_file expects 1 argument".into())) }.boxed();
    };

    async move {
        match fs::remove_file(&path_str).await {
            Ok(_) => Ok(Value::Bool(true)),
            Err(_) => Ok(Value::Bool(false)),
        }
    }.boxed()
}
