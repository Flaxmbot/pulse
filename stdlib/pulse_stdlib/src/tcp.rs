use pulse_ast::object::{HeapInterface, Object};
use pulse_ast::value::PulseSocket;
use pulse_ast::{PulseError, PulseResult, Value};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use std::future::Future;
use std::pin::Pin;

pub fn tcp_connect_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    Box::pin(async move {
        if args.is_empty() {
            return Err(PulseError::RuntimeError("Expected address string".into()));
        }

        let addr = if let Value::Obj(h) = args[0] {
            if let Some(Object::String(s)) = heap.get_object(h) {
                s.clone()
            } else {
                return Err(PulseError::RuntimeError("Expected string address".into()));
            }
        } else {
            return Err(PulseError::RuntimeError("Expected string address".into()));
        };

        match TcpStream::connect(addr).await {
            Ok(stream) => {
                let socket_obj = PulseSocket(Arc::new(Mutex::new(Some(stream))));
                let handle = heap.alloc_object(Object::Socket(socket_obj));
                Ok(Value::Obj(handle))
            }
            Err(e) => Err(PulseError::RuntimeError(format!("TCP connect failed: {}", e))),
        }
    })
}

pub fn tcp_write_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    Box::pin(async move {
        if args.len() < 2 {
            return Err(PulseError::RuntimeError("Expected socket and data".into()));
        }

        let socket_handle = if let Value::Obj(h) = args[0] {
            h
        } else {
            return Err(PulseError::RuntimeError("Expected socket object".into()));
        };

        let data_str = if let Value::Obj(h) = args[1] {
            if let Some(Object::String(s)) = heap.get_object(h) {
                s.clone()
            } else {
                return Err(PulseError::RuntimeError("Expected string data".into()));
            }
        } else {
            return Err(PulseError::RuntimeError("Expected string data".into()));
        };

        let socket = if let Some(Object::Socket(s)) = heap.get_object(socket_handle) {
            s.clone()
        } else {
            return Err(PulseError::RuntimeError("Expected socket object".into()));
        };

        let mut stream_opt = socket.0.lock().await;
        if let Some(stream) = stream_opt.as_mut() {
            match stream.write_all(data_str.as_bytes()).await {
                Ok(_) => Ok(Value::Bool(true)),
                Err(e) => Err(PulseError::RuntimeError(format!("TCP write failed: {}", e))),
            }
        } else {
            Err(PulseError::RuntimeError("TCP socket closed".into()))
        }
    })
}

pub fn tcp_read_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    Box::pin(async move {
        if args.is_empty() {
            return Err(PulseError::RuntimeError("Expected socket object".into()));
        }

        let socket_handle = if let Value::Obj(h) = args[0] {
            h
        } else {
            return Err(PulseError::RuntimeError("Expected socket object".into()));
        };

        let socket = if let Some(Object::Socket(s)) = heap.get_object(socket_handle) {
            s.clone()
        } else {
            return Err(PulseError::RuntimeError("Expected socket object".into()));
        };

        let mut stream_opt = socket.0.lock().await;
        if let Some(stream) = stream_opt.as_mut() {
            let mut buf = vec![0; 4096];
            match stream.read(&mut buf).await {
                Ok(0) => Ok(Value::Unit), // EOF
                Ok(n) => {
                    let s = String::from_utf8_lossy(&buf[..n]).to_string();
                    let handle = heap.alloc_object(Object::String(s));
                    Ok(Value::Obj(handle))
                }
                Err(e) => Err(PulseError::RuntimeError(format!("TCP read failed: {}", e))),
            }
        } else {
            Err(PulseError::RuntimeError("TCP socket closed".into()))
        }
    })
}