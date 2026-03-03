use pulse_ast::object::{HeapInterface, Object};
use pulse_ast::value::PulseWebSocket;
use pulse_ast::{PulseError, PulseResult, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use std::future::Future;
use std::pin::Pin;

pub fn websocket_connect_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    Box::pin(async move {
        if args.is_empty() {
            return Err(PulseError::RuntimeError("Expected url string".into()));
        }

        let url = if let Value::Obj(h) = args[0] {
            if let Some(Object::String(s)) = heap.get_object(h) {
                s.clone()
            } else {
                return Err(PulseError::RuntimeError("Expected string URL".into()));
            }
        } else {
            return Err(PulseError::RuntimeError("Expected string URL".into()));
        };

        match connect_async(url).await {
            Ok((ws_stream, _)) => {
                let ws_obj = PulseWebSocket(
                    Arc::new(Mutex::new(Some(ws_stream))),
                );
                let handle = heap.alloc_object(Object::WebSocket(ws_obj));
                Ok(Value::Obj(handle))
            }
            Err(e) => Err(PulseError::RuntimeError(format!("WebSocket connect failed: {}", e))),
        }
    })
}

pub fn websocket_send_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    Box::pin(async move {
        if args.len() < 2 {
            return Err(PulseError::RuntimeError("Expected websocket and message".into()));
        }

        let ws_handle = if let Value::Obj(h) = args[0] {
            h
        } else {
            return Err(PulseError::RuntimeError("Expected websocket object".into()));
        };

        let msg_str = if let Value::Obj(h) = args[1] {
            if let Some(Object::String(s)) = heap.get_object(h) {
                s.clone()
            } else {
                return Err(PulseError::RuntimeError("Expected string message".into()));
            }
        } else {
            return Err(PulseError::RuntimeError("Expected string message".into()));
        };

        let ws = if let Some(Object::WebSocket(ws)) = heap.get_object(ws_handle) {
            ws.clone()
        } else {
            return Err(PulseError::RuntimeError("Expected websocket object".into()));
        };

        let mut sender_opt = ws.0.lock().await;
        if let Some(ref mut sender) = *sender_opt {
            use futures_util::SinkExt;
            match sender.send(Message::Text(msg_str.into())).await {
                Ok(_) => Ok(Value::Bool(true)),
                Err(e) => Err(PulseError::RuntimeError(format!("WebSocket send failed: {}", e))),
            }
        } else {
            Err(PulseError::RuntimeError("WebSocket closed".into()))
        }
    })
}

pub fn websocket_recv_native<'a>(
    heap: &'a mut dyn HeapInterface,
    args: &'a [Value],
) -> Pin<Box<dyn Future<Output = PulseResult<Value>> + Send + 'a>> {
    Box::pin(async move {
        if args.is_empty() {
            return Err(PulseError::RuntimeError("Expected websocket object".into()));
        }

        let ws_handle = if let Value::Obj(h) = args[0] {
            h
        } else {
            return Err(PulseError::RuntimeError("Expected websocket object".into()));
        };

        let ws = if let Some(Object::WebSocket(ws)) = heap.get_object(ws_handle) {
            ws.clone()
        } else {
            return Err(PulseError::RuntimeError("Expected websocket object".into()));
        };

        let mut receiver_opt = ws.0.lock().await;
        if let Some(ref mut receiver) = *receiver_opt {
            use futures_util::StreamExt;
            match receiver.next().await {
                Some(Ok(msg)) => {
                    if let Message::Text(text) = msg {
                        let text_str = text.to_string();
                        let handle = heap.alloc_object(Object::String(text_str));
                        Ok(Value::Obj(handle))
                    } else {
                        Ok(Value::Unit) // Ignore non-text messages for now
                    }
                }
                Some(Err(e)) => Err(PulseError::RuntimeError(format!("WebSocket recv failed: {}", e))),
                None => Err(PulseError::RuntimeError("WebSocket closed".into())),
            }
        } else {
            Err(PulseError::RuntimeError("WebSocket closed".into()))
        }
    })
}