use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicU32, AtomicUsize, Ordering},
    Arc,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Notify, RwLock};

use crate::actor::Actor;
use crate::cluster::{create_cluster, Cluster, Node};
use crate::mailbox::{Message, SystemMessage};
use crate::network::MessageEnvelope;
use pulse_ast::object::{ObjHandle, Object};
use pulse_ast::{ActorId, Chunk, Constant, PulseError, PulseResult};
use pulse_vm::shared_heap::create_shared_heap;
use pulse_vm::VM;

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
    // Shared heap for zero-copy cross-actor communication
    shared_heap: Arc<pulse_vm::shared_heap::SharedHeap>,
    // Map ActorId to the Sender for that actor
    actors: RwLock<HashMap<ActorId, mpsc::UnboundedSender<Message>>>,

    // Registry for named actors
    registry: RwLock<HashMap<String, ActorId>>,
    // Peer connections (NodeId -> Sender to network task)
    peers: RwLock<HashMap<u64, mpsc::UnboundedSender<MessageEnvelope>>>,

    // Active actor count
    active_count: AtomicUsize,
    shutdown_notify: Notify,

    // Errors collected during runtime
    errors: RwLock<Vec<String>>,

    // Cluster management
    cluster: RwLock<Option<Cluster>>,
}

impl Runtime {
    pub fn new(node_id: u64) -> Self {
        let state = RuntimeState {
            node_id,
            next_pid: AtomicU32::new(1),
            shared_heap: create_shared_heap(),
            actors: RwLock::new(HashMap::new()),
            registry: RwLock::new(HashMap::new()),
            peers: RwLock::new(HashMap::new()),
            active_count: AtomicUsize::new(0),
            shutdown_notify: Notify::new(),
            errors: RwLock::new(Vec::new()),
            cluster: RwLock::new(None),
        };

        Self {
            handle: RuntimeHandle {
                state: Arc::new(state),
            },
        }
    }

    pub async fn run(&self) -> Result<(), String> {
        // Wait until active count is 0
        let count = self.handle.state.active_count.load(Ordering::SeqCst);
        if count > 0 {
            self.handle.state.shutdown_notify.notified().await;
        }

        // Check for errors
        let errors = self.handle.state.errors.read().await;
        if !errors.is_empty() {
            return Err(errors.join("\n"));
        }

        Ok(())
    }
}

impl RuntimeHandle {
    pub fn new_dummy() -> Self {
        let state = RuntimeState {
            node_id: 0,
            next_pid: AtomicU32::new(1),
            shared_heap: create_shared_heap(),
            actors: RwLock::new(HashMap::new()),
            registry: RwLock::new(HashMap::new()),
            peers: RwLock::new(HashMap::new()),
            active_count: AtomicUsize::new(0),
            shutdown_notify: Notify::new(),
            errors: RwLock::new(Vec::new()),
            cluster: RwLock::new(None),
        };
        Self {
            state: Arc::new(state),
        }
    }

    /// Get the shared heap for cross-actor communication
    pub fn shared_heap(&self) -> Arc<pulse_vm::shared_heap::SharedHeap> {
        self.state.shared_heap.clone()
    }

    pub fn get_actor_count(&self) -> usize {
        self.state.active_count.load(Ordering::SeqCst)
    }

    pub async fn spawn(&self, chunk: Chunk, module_path: Option<String>) -> ActorId {
        let pid_num = self.state.next_pid.fetch_add(1, Ordering::Relaxed);
        let pid = ActorId::new(self.state.node_id, pid_num);

        // Pass the shared heap to the VM for zero-copy shared memory
        let shared_heap = self.state.shared_heap.clone();
        let mut vm = VM::new(chunk, pid, Some(shared_heap));
        // Patch module path in top-level closure if it exists
        // (This logic was in old runtime, reproducing it)
        // Note: VM::new creates a closure for the chunk.
        if let Some(Object::Closure(c)) = vm.heap.get_mut(ObjHandle(0)) {
            c.function.module_path = module_path;
        }

        self.spawn_actor_process(pid, vm).await
    }

    pub async fn spawn_from_actor(
        &self,
        closure_const: Constant,
        captured_upvalues: Vec<Constant>,
        globals: std::collections::HashMap<String, Constant>,
        arguments: Vec<Constant>,
    ) -> ActorId {
        let pid_num = self.state.next_pid.fetch_add(1, Ordering::Relaxed);
        let pid = ActorId::new(self.state.node_id, pid_num);

        // Pass the shared heap to the VM for zero-copy shared memory
        let shared_heap = self.state.shared_heap.clone();
        let vm = VM::new_spawn(
            closure_const,
            captured_upvalues,
            pid,
            Some(shared_heap),
            globals,
            arguments,
        );

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
                tx.send(msg)
                    .map_err(|_| PulseError::ActorNotFound(target))?;
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
            tx.send(envelope).map_err(|e| {
                PulseError::RuntimeError(format!("Failed to send to network task: {}", e))
            })?;
            Ok(())
        } else {
            Err(PulseError::RuntimeError(format!(
                "No connection to node {}",
                target.node_id
            )))
        }
    }

    pub async fn register(&self, name: String, pid: ActorId) -> bool {
        let mut registry = self.state.registry.write().await;
        if let std::collections::hash_map::Entry::Vacant(e) = registry.entry(name) {
            e.insert(pid);
            true
        } else {
            false
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
        self.send(target, Message::System(SystemMessage::Monitor(watcher)))
            .await
    }

    pub async fn exit(&self, pid: ActorId, reason: String) {
        if reason != "normal" {
            tracing::error!("Actor {:?} exited with error: {}", pid, reason);
            eprintln!(
                "\n🚨 [PULSE RUNTIME ERROR] Actor {:?} crashed: {}\n",
                pid, reason
            );
            let mut errors = self.state.errors.write().await;
            errors.push(format!("Actor {:?} failed: {}", pid, reason));
        }
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
        let mut stream = TcpStream::connect(addr)
            .await
            .map_err(|e| PulseError::RuntimeError(format!("Connect failed: {}", e)))?;

        // Handshake
        stream
            .write_all(&self.state.node_id.to_le_bytes())
            .await
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
                        if writer.write_all(&len.to_le_bytes()).await.is_err() {
                            break;
                        }
                        if writer.write_all(&bytes).await.is_err() {
                            break;
                        }
                    }
                }
            });

            let read_task = tokio::spawn(async move {
                loop {
                    let mut len_buf = [0u8; 4];
                    if reader.read_exact(&mut len_buf).await.is_err() {
                        break;
                    }
                    let len = u32::from_le_bytes(len_buf) as usize;

                    let mut buf = vec![0u8; len];
                    if reader.read_exact(&mut buf).await.is_err() {
                        break;
                    }

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

    // ============== Cluster Management API ==============

    /// Start cluster mode on the given port
    pub async fn cluster_start(&self, port: u16) -> PulseResult<()> {
        let cluster = create_cluster(port)
            .await
            .map_err(|e| PulseError::RuntimeError(format!("Failed to start cluster: {}", e)))?;

        let mut cluster_lock = self.state.cluster.write().await;
        *cluster_lock = Some(cluster);

        tracing::info!("Cluster started on port {}", port);
        Ok(())
    }

    /// Join an existing cluster at the given address
    pub async fn cluster_join(&self, address: SocketAddr) -> PulseResult<()> {
        let cluster_lock = self.state.cluster.read().await;
        if let Some(cluster) = cluster_lock.as_ref() {
            cluster
                .join(address)
                .await
                .map_err(|e| PulseError::RuntimeError(format!("Failed to join cluster: {}", e)))?;
            tracing::info!("Joined cluster at {}", address);
            Ok(())
        } else {
            Err(PulseError::RuntimeError("Cluster not started".to_string()))
        }
    }

    /// Leave the current cluster
    pub async fn cluster_leave(&self) -> PulseResult<()> {
        let cluster_lock = self.state.cluster.read().await;
        if let Some(cluster) = cluster_lock.as_ref() {
            cluster
                .leave()
                .await
                .map_err(|e| PulseError::RuntimeError(format!("Failed to leave cluster: {}", e)))?;
            tracing::info!("Left cluster");
            Ok(())
        } else {
            Err(PulseError::RuntimeError("Cluster not started".to_string()))
        }
    }

    /// Get current node ID
    pub async fn cluster_node_id(&self) -> Option<String> {
        let cluster_lock = self.state.cluster.read().await;
        if let Some(cluster) = cluster_lock.as_ref() {
            Some(cluster.node_id().await.0)
        } else {
            None
        }
    }

    /// Get all cluster members
    pub async fn cluster_members(&self) -> Vec<Node> {
        let cluster_lock = self.state.cluster.read().await;
        if let Some(cluster) = cluster_lock.as_ref() {
            cluster.members().await
        } else {
            Vec::new()
        }
    }

    /// Get cluster member count
    pub async fn cluster_member_count(&self) -> usize {
        let cluster_lock = self.state.cluster.read().await;
        if let Some(cluster) = cluster_lock.as_ref() {
            cluster.member_count().await
        } else {
            0
        }
    }

    /// Check if this node is part of a cluster
    pub async fn is_clustered(&self) -> bool {
        let cluster_lock = self.state.cluster.read().await;
        if let Some(cluster) = cluster_lock.as_ref() {
            cluster.is_clustered().await
        } else {
            false
        }
    }
}
