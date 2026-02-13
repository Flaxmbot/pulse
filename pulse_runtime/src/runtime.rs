
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, atomic::{AtomicU32, AtomicUsize, Ordering}};
use tokio::sync::{mpsc, RwLock, Notify};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use pulse_core::{ActorId, PulseError, PulseResult, Value, Chunk, Constant};
use pulse_core::object::{Object, ObjHandle, Closure};
use pulse_vm::VM;
use crate::actor::{Actor, ActorStatus};
use crate::mailbox::{Message, SystemMessage};
use crate::network::MessageEnvelope;

pub struct Runtime {
    pub handle: RuntimeHandle,
}

#[derive(Clone)]
pub struct RuntimeHandle {
    state: Arc<RuntimeState>,
}

struct RuntimeState {
    node_id: u64,
    next_pid: AtomicU32,
    // Map ActorId to the Sender for that actor
    actors: RwLock<HashMap<ActorId, mpsc::UnboundedSender<Message>>>,

    // Registry for named actors
    registry: RwLock<HashMap<String, ActorId>>,
    // Peer connections (NodeId -> Sender to network task)
    peers: RwLock<HashMap<u64, mpsc::UnboundedSender<MessageEnvelope>>>,
    
    // Active actor count
    active_count: AtomicUsize,
    shutdown_notify: Notify,
}

impl Runtime {
    pub fn new(node_id: u64) -> Self {
        let state = RuntimeState {
            node_id,
            next_pid: AtomicU32::new(1),
            actors: RwLock::new(HashMap::new()),
            registry: RwLock::new(HashMap::new()),
            peers: RwLock::new(HashMap::new()),
            active_count: AtomicUsize::new(0),
            shutdown_notify: Notify::new(),
        };
        
        Self {
            handle: RuntimeHandle {
                state: Arc::new(state),
            }
        }
    }
    
    pub async fn run(&self) {
        // Wait until active count is 0
        // If it starts at 0, we should probably check immediately?
        // But usually we spawn at least one before running.
        let count = self.handle.state.active_count.load(Ordering::SeqCst);
        if count == 0 {
             return;
        }
        
        self.handle.state.shutdown_notify.notified().await;
    }
}

impl RuntimeHandle {

    pub fn new_dummy() -> Self {
         let state = RuntimeState {
            node_id: 0,
            next_pid: AtomicU32::new(1),
            actors: RwLock::new(HashMap::new()),
            registry: RwLock::new(HashMap::new()),
            peers: RwLock::new(HashMap::new()),
            active_count: AtomicUsize::new(0),
            shutdown_notify: Notify::new(),
        };
        Self { state: Arc::new(state) }
    }

    pub async fn spawn(&self, chunk: Chunk, module_path: Option<String>) -> ActorId {
        let pid_num = self.state.next_pid.fetch_add(1, Ordering::Relaxed);
        let pid = ActorId::new(self.state.node_id, pid_num);
        
        let mut vm = VM::new(chunk, pid);
        // Patch module path in top-level closure if it exists
        // (This logic was in old runtime, reproducing it)
        // Note: VM::new creates a closure for the chunk.
        if let Some(Object::Closure(c)) = vm.heap.get_mut(ObjHandle(0)) {
            c.function.module_path = module_path;
        }

        self.spawn_actor_process(pid, vm).await
    }
    
    pub async fn spawn_from_actor(&self, chunk: Arc<Chunk>, ip: usize) -> ActorId {
        let pid_num = self.state.next_pid.fetch_add(1, Ordering::Relaxed);
        let pid = ActorId::new(self.state.node_id, pid_num);
        
        // VM::new_spawn creates a new VM sharing constants etc (via checking Arc<Chunk>)
        // But here we clone the Arc<Chunk>
        let vm = VM::new_spawn(chunk, pid, ip);
        
        self.spawn_actor_process(pid, vm).await
    }
    
    
    async fn spawn_actor_process(&self, pid: ActorId, vm: VM) -> ActorId {
        let (tx, rx) = mpsc::unbounded_channel();
        

        {
            let mut actors = self.state.actors.write().await;
            actors.insert(pid, tx);
        }
        
        self.state.active_count.fetch_add(1, Ordering::SeqCst);
        
        let actor = Actor::new(pid, vm, self.clone());
        let state = self.state.clone();
        
        tokio::spawn(async move {
            actor.run(rx).await;
            
            // Cleanup
            let prev = state.active_count.fetch_sub(1, Ordering::SeqCst);
            if prev == 1 {
                state.shutdown_notify.notify_waiters();
            }
            
            // Remove from actors map if not already removed?
            // Actor::run usually ends when VM halts.
            // We should remove from actors map to close channel.
            let mut actors = state.actors.write().await;
            actors.remove(&pid);
        });
        
        pid
    }
    
    pub async fn send(&self, target: ActorId, msg: Message) -> PulseResult<()> {
        if target.node_id == self.state.node_id {
            let actors = self.state.actors.read().await;
            if let Some(tx) = actors.get(&target) {
                tx.send(msg).map_err(|_| PulseError::ActorNotFound(target))?;
                Ok(())
            } else {
                Err(PulseError::ActorNotFound(target))
            }
        } else {
            // Remote send
            self.send_remote(target, msg).await
        }
    }
    
    async fn send_remote(&self, target: ActorId, msg: Message) -> PulseResult<()> {
        let envelope = MessageEnvelope::new(target, None, msg);
        let peers = self.state.peers.read().await;
        
        if let Some(tx) = peers.get(&target.node_id) {
            tx.send(envelope).map_err(|e| PulseError::RuntimeError(format!("Failed to send to network task: {}", e)))?;
            Ok(())
        } else {
            Err(PulseError::RuntimeError(format!("No connection to node {}", target.node_id)))
        }
    }
    
    pub async fn register(&self, name: String, pid: ActorId) -> bool {
        let mut registry = self.state.registry.write().await;
        if registry.contains_key(&name) {
            false
        } else {
            registry.insert(name, pid);
            true
        }
    }
    
    pub async fn unregister(&self, name: &str) {
        let mut registry = self.state.registry.write().await;
        registry.remove(name);
    }
    
    pub async fn whereis(&self, name: &str) -> Option<ActorId> {
        let registry = self.state.registry.read().await;
        registry.get(name).cloned()
    }
    
    pub async fn monitor(&self, watcher: ActorId, target: ActorId) -> PulseResult<()> {
         // In async model, monitor is tricky.
         // Calling "Link" or "Monitor" on target actor via message is best.
         // We send a SystemMessage::Monitor(watcher) to target.
         self.send(target, Message::System(SystemMessage::Monitor(watcher))).await
    }
    
    pub async fn exit(&self, pid: ActorId, reason: String) {
        // Remove from registry
        {
            let mut actors = self.state.actors.write().await;
            actors.remove(&pid);
        }
        
        // TODO: Handle Registry cleanup (reverse lookup needed?)
        // TODO: Notify links/monitors? 
        // The Actor itself should probably do this before dying or we need a central "Death Handler".
        // In the actor loop, it handles its own death notifications (links/monitors).
        // But if we remove it from `actors` map, no one can send to it.
    }
    
    // Low-level network connection
    pub async fn connect_peer(&self, node_id: u64, addr: &str) -> PulseResult<()> {
        let mut stream = TcpStream::connect(addr).await
            .map_err(|e| PulseError::RuntimeError(format!("Connect failed: {}", e)))?;
            
        // Handshake
        stream.write_all(&self.state.node_id.to_le_bytes()).await
            .map_err(|e| PulseError::RuntimeError(format!("Handshake failed: {}", e)))?;
            
        // Spawn network handler for this connection
        let (tx, mut rx) = mpsc::unbounded_channel::<MessageEnvelope>();
        
        {
            let mut peers = self.state.peers.write().await;
            peers.insert(node_id, tx);
        }
        
        let handle = self.clone();
        tokio::spawn(async move {
            // Write loop
            // We need split logic or select!
            let (mut reader, mut writer) = stream.into_split();
            
            // Allow concurrent read/write
             let write_task = tokio::spawn(async move {
                 while let Some(env) = rx.recv().await {
                     // Serialize
                     // TODO: envelope.to_bytes() is synchronous and uses bincode. 
                     // It returns Result<Vec<u8>, ...>
                     if let Ok(bytes) = env.to_bytes() {
                         let len = bytes.len() as u32;
                         if writer.write_all(&len.to_le_bytes()).await.is_err() { break; }
                         if writer.write_all(&bytes).await.is_err() { break; }
                     }
                 }
             });
             
             let read_task = tokio::spawn(async move {
                 loop {
                     let mut len_buf = [0u8; 4];
                     if reader.read_exact(&mut len_buf).await.is_err() { break; }
                     let len = u32::from_le_bytes(len_buf) as usize;
                     
                     let mut buf = vec![0u8; len];
                     if reader.read_exact(&mut buf).await.is_err() { break; }
                     
                     if let Ok(env) = MessageEnvelope::from_bytes(&buf) {
                         // Deliver to local actor
                         let _ = handle.send(env.target, env.message).await;
                     }
                 }
             });
             
             let _ = tokio::join!(write_task, read_task);
        });
        
        Ok(())
    }
}
