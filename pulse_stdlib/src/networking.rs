//! Networking native functions

use pulse_core::{Value, PulseResult, PulseError};
use pulse_core::object::{HeapInterface, Object, PulseSocket, PulseListener};
use std::pin::Pin;
use std::future::Future;
use futures::FutureExt;
use std::sync::Arc;
use tokio::net::{TcpListener, UdpSocket};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// tcp_connect(host: String, port: Int) -> Socket
/// Connects to a TCP server and returns a Socket object (wrapped in Value)
pub fn tcp_connect_native<'a>(heap: &'a mut dyn HeapInterface, args: &'a [Value]) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 2 {
            return Err(PulseError::RuntimeError("tcp_connect(host, port) expects 2 arguments".into()));
        }

        // We need to access heap to get the string.
        let host = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into()
                    });
                }
            }
            _ => return Err(PulseError::TypeMismatch {
                expected: "string".into(),
                got: args[0].type_name()
            }),
        };

        let port = args[1].as_int()?;
        if port < 0 || port > 65535 {
            return Err(PulseError::RuntimeError("Port must be between 0 and 65535".into()));
        }

        let addr = format!("{}:{}", host, port);
        
        match tokio::net::TcpStream::connect(&addr).await {
            Ok(stream) => {
                let handle = heap.alloc_object(Object::Socket(PulseSocket(Arc::new(tokio::sync::Mutex::new(stream)))));
                Ok(Value::Obj(handle))
            }
            Err(e) => Err(PulseError::RuntimeError(format!("Failed to connect to {}: {}", addr, e))),
        }
    }.boxed()
}

/// tcp_listen(port: Int) -> Listener
/// Starts listening on a TCP port and returns a Listener object.
pub fn tcp_listen_native<'a>(heap: &'a mut dyn HeapInterface, args: &'a [Value]) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 1 {
            return Err(PulseError::RuntimeError("tcp_listen(port) expects 1 argument".into()));
        }

        let port = args[0].as_int()?;
        if port < 0 || port > 65535 {
            return Err(PulseError::RuntimeError("Port must be between 0 and 65535".into()));
        }

        let addr = format!("0.0.0.0:{}", port);
        
        match TcpListener::bind(&addr).await {
            Ok(listener) => {
                let handle = heap.alloc_object(Object::Listener(PulseListener(Arc::new(listener))));
                Ok(Value::Obj(handle))
            }
            Err(e) => Err(PulseError::RuntimeError(format!("Failed to bind to {}: {}", addr, e))),
        }
    }.boxed()
}

/// tcp_accept(listener: Listener) -> Socket
/// Accepts a connection from a Listener and returns a Socket.
pub fn tcp_accept_native<'a>(heap: &'a mut dyn HeapInterface, args: &'a [Value]) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 1 {
            return Err(PulseError::RuntimeError("tcp_accept(listener) expects 1 argument".into()));
        }

        let listener = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::Listener(l)) = heap.get_object(*h) {
                    l.0.clone()
                } else {
                     return Err(PulseError::TypeMismatch {
                        expected: "listener".into(),
                        got: "object".into()
                    });
                }
            }
             _ => return Err(PulseError::TypeMismatch {
                expected: "listener".into(),
                got: args[0].type_name()
            }),
        };

        match listener.accept().await {
            Ok((stream, _addr)) => {
                let handle = heap.alloc_object(Object::Socket(PulseSocket(Arc::new(tokio::sync::Mutex::new(stream)))));
                Ok(Value::Obj(handle))
            }
            Err(e) => Err(PulseError::RuntimeError(format!("Accept failed: {}", e))),
        }
    }.boxed()
}

/// tcp_send(socket: Socket, data: String) -> Bool
/// Sends data over a TCP socket
pub fn tcp_send_native<'a>(heap: &'a mut dyn HeapInterface, args: &'a [Value]) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 2 {
            return Err(PulseError::RuntimeError("tcp_send(socket, data) expects 2 arguments".into()));
        }

        let socket = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::Socket(s)) = heap.get_object(*h) {
                    s.0.clone()
                } else {
                     return Err(PulseError::TypeMismatch { expected: "socket".into(), got: "object".into() });
                }
            }
             _ => return Err(PulseError::TypeMismatch { expected: "socket".into(), got: args[0].type_name() }),
        };

        let data = match &args[1] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                     return Err(PulseError::TypeMismatch { expected: "string".into(), got: "object".into() });
                }
            }
            _ => return Err(PulseError::TypeMismatch { expected: "string".into(), got: args[1].type_name() }),
        };

        let mut socket_lock = socket.lock().await;
        match socket_lock.write_all(data.as_bytes()).await {
            Ok(_) => Ok(Value::Bool(true)),
            Err(e) => Err(PulseError::RuntimeError(format!("Send failed: {}", e))),
        }
    }.boxed()
}

/// tcp_receive(socket: Socket, count: Int) -> String
/// Receives up to `count` bytes from socket (or until EOF)
pub fn tcp_receive_native<'a>(heap: &'a mut dyn HeapInterface, args: &'a [Value]) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 2 {
            return Err(PulseError::RuntimeError("tcp_receive(socket, count) expects 2 arguments".into()));
        }

        let socket = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::Socket(s)) = heap.get_object(*h) {
                    s.0.clone()
                } else {
                     return Err(PulseError::TypeMismatch { expected: "socket".into(), got: "object".into() });
                }
            }
             _ => return Err(PulseError::TypeMismatch { expected: "socket".into(), got: args[0].type_name() }),
        };

        let count = match args[1].as_int() {
            Ok(i) => i as usize,
            Err(_) => return Err(PulseError::TypeMismatch { expected: "int".into(), got: args[1].type_name() }),
        };

        if count == 0 { return Ok(Value::Unit); }

        let mut buf = vec![0u8; count];
        let mut socket_lock = socket.lock().await;
        
        match socket_lock.read(&mut buf).await {
            Ok(n) => {
                let s = String::from_utf8_lossy(&buf[..n]).to_string();
                let handle = heap.alloc_object(Object::String(s));
                Ok(Value::Obj(handle))
            }
            Err(e) => Err(PulseError::RuntimeError(format!("Receive failed: {}", e))),
        }
    }.boxed()
}

/// http_get(url: String) -> String
/// Performs an HTTP GET request and returns the response body
pub fn http_get_native<'a>(heap: &'a mut dyn HeapInterface, args: &'a [Value]) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 1 {
            return Err(PulseError::RuntimeError("http_get expects 1 argument".into()));
        }

        let url = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into()
                    });
                }
            }
            _ => return Err(PulseError::TypeMismatch {
                expected: "string".into(),
                got: args[0].type_name()
            }),
        };

        let response = reqwest::get(&url).await
            .map_err(|e| PulseError::RuntimeError(format!("HTTP Request Failed: {}", e)))?
            .text().await
            .map_err(|e| PulseError::RuntimeError(format!("Failed to read text: {}", e)))?;

        let handle = heap.alloc_object(Object::String(response));
        Ok(Value::Obj(handle))
    }.boxed()
}

/// http_post(url: String, data: String) -> String
/// Performs an HTTP POST request and returns the response body
pub fn http_post_native<'a>(heap: &'a mut dyn HeapInterface, args: &'a [Value]) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 2 {
            return Err(PulseError::RuntimeError("http_post expects 2 arguments".into()));
        }

        let url = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into()
                    });
                }
            }
            _ => return Err(PulseError::TypeMismatch {
                expected: "string".into(),
                got: args[0].type_name()
            }),
        };

        let data = match &args[1] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into()
                    });
                }
            }
            _ => return Err(PulseError::TypeMismatch {
                expected: "string".into(),
                got: args[1].type_name()
            }),
        };

        let client = reqwest::Client::new();
        let response = client.post(&url)
            .body(data)
            .send().await
            .map_err(|e| PulseError::RuntimeError(format!("HTTP Request Failed: {}", e)))?
            .text().await
            .map_err(|e| PulseError::RuntimeError(format!("Failed to read text: {}", e)))?;

        let handle = heap.alloc_object(Object::String(response));
        Ok(Value::Obj(handle))
    }.boxed()
}

/// socket_create(address: String) -> Unit (UDP Socket placeholder)
/// Creates a UDP socket bound to the given address
pub fn socket_create_native<'a>(heap: &'a mut dyn HeapInterface, args: &'a [Value]) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 1 {
            return Err(PulseError::RuntimeError("socket_create expects 1 argument".into()));
        }

        let addr = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into()
                    });
                }
            }
            _ => return Err(PulseError::TypeMismatch {
                expected: "string".into(),
                got: args[0].type_name()
            }),
        };

        match UdpSocket::bind(&addr).await {
            Ok(_socket) => {
                // TODO: Store UdpSocket in Object::UdpSocket?
                Ok(Value::Unit)
            }
            Err(e) => Err(PulseError::RuntimeError(format!("Failed to create socket: {}", e))),
        }
    }.boxed()
}

/// dns_resolve(hostname: String) -> List
/// Resolves a hostname to IP addresses
pub fn dns_resolve_native<'a>(heap: &'a mut dyn HeapInterface, args: &'a [Value]) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    let args = args.to_vec();
    async move {
        if args.len() != 1 {
            return Err(PulseError::RuntimeError("dns_resolve expects 1 argument".into()));
        }

        let hostname = match &args[0] {
            Value::Obj(h) => {
                if let Some(Object::String(s)) = heap.get_object(*h) {
                    s.clone()
                } else {
                    return Err(PulseError::TypeMismatch {
                        expected: "string".into(),
                        got: "object".into()
                    });
                }
            }
            _ => return Err(PulseError::TypeMismatch {
                expected: "string".into(),
                got: args[0].type_name()
            }),
        };

        match tokio::net::lookup_host(format!("{}:80", hostname)).await {
            Ok(iter) => {
                let ips: Vec<Value> = iter.map(|addr| {
                    let ip_str = addr.ip().to_string();
                    let handle = heap.alloc_object(Object::String(ip_str));
                    Value::Obj(handle)
                }).collect();
                
                let handle = heap.alloc_object(Object::List(ips));
                Ok(Value::Obj(handle))
            },
            Err(e) => Err(PulseError::RuntimeError(format!("DNS resolution failed: {}", e))),
        }
    }.boxed()
}