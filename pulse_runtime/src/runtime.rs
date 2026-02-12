use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::thread;

use pulse_core::{ActorId, PulseError, PulseResult, Value, Chunk};
use pulse_core::object::{Object, ObjHandle, Closure};
use pulse_vm::{VM, VMStatus, CallFrame};
use crate::actor::{Actor, ActorStatus};
use crate::mailbox::Message;
use crate::network::MessageEnvelope;

pub struct Runtime {
    actors: HashMap<ActorId, Arc<Mutex<Actor>>>,
    run_queue: VecDeque<ActorId>,
    next_pid: u64,
    pub node_id: u128,
    peer_connections: HashMap<u128, Arc<Mutex<TcpStream>>>,
    remote_queue: Arc<Mutex<VecDeque<MessageEnvelope>>>,
    registry: HashMap<String, ActorId>,
    reverse_registry: HashMap<ActorId, HashSet<String>>,
    last_error: Option<String>,
}

impl Runtime {
    pub fn new(node_id: u128) -> Self {
        Self {
            actors: HashMap::new(),
            run_queue: VecDeque::new(),
            next_pid: 1,
            node_id,
            peer_connections: HashMap::new(),
            remote_queue: Arc::new(Mutex::new(VecDeque::new())),
            registry: HashMap::new(),
            reverse_registry: HashMap::new(),
            last_error: None,
        }
    }
    
    /// Get and clear the last runtime error (for test framework)
    pub fn get_last_error(&mut self) -> Option<String> {
        self.last_error.take()
    }

    pub fn spawn(&mut self, chunk: Chunk, module_path: Option<String>) -> ActorId {
        let pid = ActorId::new(self.node_id, self.next_pid);
        self.next_pid += 1;

        let mut vm = VM::new(chunk, pid);
        if let Some(Object::Closure(c)) = vm.heap.get_mut(ObjHandle(0)) {
            c.function.module_path = module_path;
        }

        let actor = Actor::new(pid, vm);

        self.actors.insert(pid, Arc::new(Mutex::new(actor)));
        self.run_queue.push_back(pid);
        // The active_actors counter is calculated dynamically now, so we don't need to maintain it
        pid
    }

    pub fn spawn_with_rc(&mut self, chunk: Arc<Chunk>, pid: ActorId, ip: usize) -> ActorId {
        let vm = VM::new_spawn(chunk.clone(), pid, ip);

        let actor = Actor::new(pid, vm);

        self.actors.insert(pid, Arc::new(Mutex::new(actor)));
        self.run_queue.push_back(pid);
        // The active_actors counter is calculated dynamically now, so we don't need to maintain it
        pid
    }

    pub fn send(&mut self, target: ActorId, msg: Message) -> PulseResult<()> {
        if target.node_id == self.node_id {
            if let Some(actor_ref) = self.actors.get(&target) {
                let mut actor = actor_ref.lock().unwrap();
                if actor.status == ActorStatus::Waiting {
                    actor.deliver_message(msg);
                    self.run_queue.push_back(target);
                } else {
                    actor.mailbox.push(msg);
                }
                Ok(())
            } else {
                Err(PulseError::ActorNotFound(target))
            }
        } else {
            // Remote Send
            self.send_remote(target, msg)
        }
    }

    fn send_remote(&mut self, target: ActorId, msg: Message) -> PulseResult<()> {
        if let Some(stream_lock) = self.peer_connections.get(&target.node_id) {
            let envelope = MessageEnvelope::new(target, None, msg);
            let bytes = envelope.to_bytes().map_err(|e| PulseError::RuntimeError(format!("Serialization error: {}", e)))?;
            
            let mut stream = stream_lock.lock().unwrap();
            // Send length prefix (v0)
            let len = bytes.len() as u32;
            stream.write_all(&len.to_le_bytes()).map_err(|e| PulseError::RuntimeError(format!("Network error: {}", e)))?;
            stream.write_all(&bytes).map_err(|e| PulseError::RuntimeError(format!("Network error: {}", e)))?;
            Ok(())
        } else {
            Err(PulseError::RuntimeError(format!("No connection to node {}", target.node_id)))
        }
    }

    pub fn connect(&mut self, node_id: u128, addr: &str) -> PulseResult<()> {
        let mut stream = TcpStream::connect(addr).map_err(|e| PulseError::RuntimeError(format!("Failed to connect to node {}: {}", node_id, e)))?;
        // Handshake: Send local NodeId
        stream.write_all(&self.node_id.to_le_bytes()).map_err(|e| PulseError::RuntimeError(format!("Handshake failed: {}", e)))?;
        
        self.peer_connections.insert(node_id, Arc::new(Mutex::new(stream)));
        Ok(())
    }

    pub fn step(&mut self) -> bool {
        // Handle remote messages first
        let messages: Vec<MessageEnvelope> = {
            let mut remote = self.remote_queue.lock().unwrap();
            remote.drain(..).collect()
        };

        for envelope in messages {
            let _ = self.send(envelope.target, envelope.message);
        }

        if let Some(pid) = self.run_queue.pop_front() {
             let actor_ref = match self.actors.get(&pid) {
                 Some(r) => r.clone(),
                 None => return true, 
             };

             let status = {
                let mut actor = actor_ref.lock().unwrap();
                if actor.status == ActorStatus::Runnable {
                    actor.vm.run(1000)  // Increased from 100 to 1000 for better performance
                } else {
                    VMStatus::Yielded
                }
             };

             match status {
                 VMStatus::Running => {
                     self.run_queue.push_back(pid);
                 },
                 VMStatus::Yielded => {
                     self.run_queue.push_back(pid);
                 },
                  VMStatus::Blocked => {
                      let mut actor = actor_ref.lock().unwrap();
                      if let Some(msg) = actor.mailbox.pop() {
                          actor.deliver_message(msg);
                          self.run_queue.push_back(pid);
                      } else {
                          actor.status = ActorStatus::Waiting;
                      }
                  },
                 VMStatus::Send { target, msg } => {
                     let _ = self.send(target, Message::User(msg));
                     self.run_queue.push_back(pid);
                 },
                 VMStatus::Spawn(offset) => {
                     let chunk_rc = {
                         let actor = actor_ref.lock().unwrap();
                         actor.vm.get_current_chunk()
                     };
                     let child_pid = ActorId::new(self.node_id, self.next_pid);
                     self.next_pid += 1;
                     self.spawn_with_rc(chunk_rc, child_pid, offset);
                     
                     {
                         let mut actor = actor_ref.lock().unwrap();
                         actor.vm.push(Value::Pid(child_pid));
                     }
                     self.run_queue.push_back(pid);
                 },
                 VMStatus::Import(path) => {
                      if path.starts_with("std/") {
                          let mut actor = actor_ref.lock().unwrap();
                          if let Some(handle) = crate::stdlib::load_std_module(&path, &mut actor.vm) {
                              actor.vm.push(pulse_core::Value::Obj(handle));
                              // Cache it
                              actor.vm.loaded_modules.insert(path, handle);
                              self.run_queue.push_back(pid);
                          } else {
                              actor.status = ActorStatus::Terminated;
                              println!("Actor {:?} failed to load std module: {}", pid, path);
                          }
                          // The `continue` here effectively means "finish processing this VMStatus and return true from step"
                          // as `step` processes one actor at a time.
                          return true; 
                      }
                     match std::fs::read_to_string(&path) {
                         Ok(source) => {
                             match pulse_compiler::compile(&source, Some(path.clone())) {
                               Ok(chunk) => {
                                   let mut actor = actor_ref.lock().unwrap();
                                   
                                   let function = pulse_core::object::Function {
                                       arity: 0,
                                       chunk: Arc::new(chunk),
                                       name: format!("module_{}", path),
                                       upvalue_count: 0,
                                       module_path: Some(path.clone()),
                                   };
                                   let closure = Closure {
                                       function,
                                       upvalues: Vec::new(),
                                   };
                                   let closure_handle = actor.vm.heap.alloc(Object::Closure(closure));
                                   
                                   // Swap globals for the module
                                   let current_globals = actor.vm.globals.clone();
                                   actor.vm.globals.clear();
                                   
                                   let frame = CallFrame {
                                       closure: closure_handle,
                                       ip: 0,
                                       stack_start: actor.vm.stack.len(),
                                       is_module: true,
                                       module_path: Some(path),
                                       prev_globals: Some(current_globals),
                                   };
                                   actor.vm.frames.push(frame);
                                   self.run_queue.push_back(pid);
                               },
                                 Err(e) => {
                                     let mut actor = actor_ref.lock().unwrap();
                                     actor.status = ActorStatus::Terminated;
                                     println!("Actor {:?} module compile error: {}", pid, e);
                                 }
                             }
                         }
                         Err(e) => {
                             let mut actor = actor_ref.lock().unwrap();
                             actor.status = ActorStatus::Terminated;
                             println!("Actor {:?} module read error: {}: {}", pid, path, e);
                         }
                     }
                 },
                 VMStatus::Link(target) => {
                     // Add target to current actor's links
                     {
                         let mut actor = actor_ref.lock().unwrap();
                         actor.links.insert(target);
                     }
                     // Also add current actor to target's links (bidirectional)
                     if let Some(target_actor_ref) = self.actors.get(&target) {
                         let mut target_actor = target_actor_ref.lock().unwrap();
                         target_actor.links.insert(pid);
                     }
                     self.run_queue.push_back(pid);
                 },
                 VMStatus::Monitor(target) => {
                     let mut target_exists = false;
                     let mut target_dead = false;
                     
                     if let Some(target_actor_ref) = self.actors.get(&target) {
                         target_exists = true;
                         let mut target_actor = target_actor_ref.lock().unwrap();
                         if target_actor.status == ActorStatus::Terminated {
                             target_dead = true;
                         } else {
                             target_actor.monitors.insert(pid);
                         }
                     }
                     
                     if !target_exists || target_dead {
                          // Send DOWN message immediately
                          let msg = Message::System(crate::mailbox::SystemMessage::MonitorExit(target, "No Process".to_string()));
                          if let Some(actor_ref) = self.actors.get(&pid) {
                              let mut actor = actor_ref.lock().unwrap();
                              if actor.status == ActorStatus::Waiting {
                                  actor.deliver_message(msg);
                                  self.run_queue.push_back(pid); // Re-queue self
                              } else {
                                  actor.mailbox.push(msg);
                              }
                          }
                     } else {
                         // Successfully monitored, just re-queue sself
                         self.run_queue.push_back(pid);
                     }
                 },
                  VMStatus::Register(name, target_pid) => {
                      let success = self.register_actor(name, target_pid);
                      {
                          let mut actor = actor_ref.lock().unwrap();
                          actor.vm.push(Value::Bool(success));
                      }
                      self.run_queue.push_back(pid);
                  },
                  VMStatus::Unregister(name) => {
                      self.unregister_name(&name);
                      {
                          let mut actor = actor_ref.lock().unwrap();
                          actor.vm.push(Value::Unit);
                      }
                      self.run_queue.push_back(pid);
                  },
                  VMStatus::WhereIs(name) => {
                      let result = self.whereis(&name);
                      {
                          let mut actor = actor_ref.lock().unwrap();
                          match result {
                              Some(target_pid) => actor.vm.push(Value::Pid(target_pid)),
                              None => actor.vm.push(Value::Unit),
                          }
                      }
                      self.run_queue.push_back(pid);
                  },
                 VMStatus::SpawnLink(offset) => {
                     let chunk_rc = {
                         let actor = actor_ref.lock().unwrap();
                         actor.vm.get_current_chunk()
                     };
                     let child_pid = ActorId::new(self.node_id, self.next_pid);
                     self.next_pid += 1;
                     self.spawn_with_rc(chunk_rc, child_pid, offset);

                     // Link parent and child
                     {
                         let mut parent_actor = actor_ref.lock().unwrap();
                         parent_actor.links.insert(child_pid);
                     }
                     if let Some(child_actor_ref) = self.actors.get(&child_pid) {
                         let mut child_actor = child_actor_ref.lock().unwrap();
                         child_actor.links.insert(pid);
                     }

                     {
                         let mut actor = actor_ref.lock().unwrap();
                         actor.vm.push(Value::Pid(child_pid));
                     }
                     self.run_queue.push_back(pid);
                 },
                 VMStatus::Paused => {
                     // Debug: actor hit breakpoint, keep in queue but don't mark terminated
                     self.run_queue.push_back(pid);
                 },
                 VMStatus::Halted => {
                     let mut actor = actor_ref.lock().unwrap();
                     if actor.status != ActorStatus::Terminated {
                         actor.status = ActorStatus::Terminated;
                     }
                     drop(actor); // Release lock before calling propagate_exit
                     self.propagate_exit(pid, "normal".to_string());
                 },
                 VMStatus::Error(e) => {
                     let mut actor = actor_ref.lock().unwrap();
                     if actor.status != ActorStatus::Terminated {
                         actor.status = ActorStatus::Terminated;
                     }
                     let error_msg = format!("{}", e);
                     self.last_error = Some(error_msg.clone());
                     drop(actor); // Release lock before calling propagate_exit
                     self.propagate_exit(pid, error_msg);
                 }
             }
             true
        } else {
            // Calculate active actors dynamically to avoid counter drift
            let mut live_actor_count = 0;
            for actor_ref in self.actors.values() {
                let actor = actor_ref.lock().unwrap();
                if actor.status != ActorStatus::Terminated {
                    live_actor_count += 1;
                }
            }
            
            let has_live_actors = live_actor_count > 0;
            let has_queued_actors = !self.run_queue.is_empty();
            
            // Check for pending remote messages
            let has_pending_messages = {
                let remote = self.remote_queue.lock().unwrap();
                !remote.is_empty()
            };
            
            if has_live_actors || has_queued_actors || has_pending_messages {
                true  // Continue running
            } else {
                false  // Safe to exit
            }
        }
    }

    pub fn get_actor_vm(&self, pid: ActorId) -> Option<Arc<Mutex<Actor>>> {
        self.actors.get(&pid).cloned()
    }

    pub fn actor_count(&self) -> usize {
        self.actors.len()
    }

    pub fn add_peer(&mut self, node_id: u128, stream: TcpStream) {
        self.peer_connections.insert(node_id, Arc::new(Mutex::new(stream)));
    }

    pub fn start_listener(&mut self, addr: &str) -> PulseResult<()> {
        let listener = TcpListener::bind(addr).map_err(|e| PulseError::RuntimeError(format!("Failed to bind to {}: {}", addr, e)))?;
        let remote_queue = self.remote_queue.clone();
        
        thread::spawn(move || {
            for stream in listener.incoming() {
                if let Ok(mut stream) = stream {
                    let remote_queue = remote_queue.clone();
                    thread::spawn(move || {
                        // Handshake: Receive NodeId
                        let mut buf = [0u8; 16];
                        if stream.read_exact(&mut buf).is_ok() {
                            let _remote_node_id = u128::from_le_bytes(buf);
                            // TODO: Add to peers if bidirectional desired
                            
                            loop {
                                // Read length prefix
                                let mut len_buf = [0u8; 4];
                                if stream.read_exact(&mut len_buf).is_err() { break; }
                                let len = u32::from_le_bytes(len_buf) as usize;
                                
                                // Read message
                                let mut msg_buf = vec![0u8; len];
                                if stream.read_exact(&mut msg_buf).is_err() { break; }
                                
                                if let Ok(envelope) = MessageEnvelope::from_bytes(&msg_buf) {
                                    remote_queue.lock().unwrap().push_back(envelope);
                                }
                            }
                        }
                    });
                }
            }
        });
        Ok(())
    }

    // Registry Methods
    pub fn register_actor(&mut self, name: String, pid: ActorId) -> bool {
        if self.registry.contains_key(&name) {
            return false;
        }
        
        self.registry.insert(name.clone(), pid);
        self.reverse_registry
            .entry(pid)
            .or_insert_with(HashSet::new)
            .insert(name);
            
        true
    }
    
    pub fn unregister_name(&mut self, name: &str) {
        if let Some(pid) = self.registry.remove(name) {
            if let Some(names) = self.reverse_registry.get_mut(&pid) {
                names.remove(name);
                if names.is_empty() {
                    self.reverse_registry.remove(&pid);
                }
            }
        }
    }
    
    pub fn whereis(&self, name: &str) -> Option<ActorId> {
        self.registry.get(name).cloned()
    }
    
    fn cleanup_actor_registry(&mut self, pid: ActorId) {
        if let Some(names) = self.reverse_registry.remove(&pid) {
            for name in names {
                self.registry.remove(&name);
            }
        }
    }

    fn propagate_exit(&mut self, pid: ActorId, reason: String) {
        // Cleanup registry for dead actor
        self.cleanup_actor_registry(pid);

        // Collect linked actors first to avoid borrowing issues
        let mut linked_actors = Vec::new();
        let mut monitoring_actors = Vec::new();
        
        // First pass: collect actors that need to be affected
        for (_, actor_ref) in self.actors.iter() {
            let actor_info = {
                let actor = actor_ref.lock().unwrap();
                (actor.id, actor.links.contains(&pid), actor.monitors.contains(&pid), actor.trapping_exits)
            };
            
            let (actor_pid, is_linked, is_monitoring, trapping_exits) = actor_info;
            
            if is_linked {
                linked_actors.push((actor_pid, trapping_exits));
            }
            
            if is_monitoring {
                monitoring_actors.push(actor_pid);
            }
        }
        
        // Handle linked actors
        for (actor_pid, trapping) in linked_actors {
            if trapping {
                // Send exit message instead of terminating
                let exit_msg = Message::System(crate::mailbox::SystemMessage::Exit(pid, reason.clone()));
                if let Some(target_actor_ref) = self.actors.get(&actor_pid) {
                    let mut target_actor = target_actor_ref.lock().unwrap();
                    if target_actor.status == ActorStatus::Waiting {
                        target_actor.deliver_message(exit_msg);
                        self.run_queue.push_back(actor_pid);
                    } else {
                        target_actor.mailbox.push(exit_msg);
                    }
                }
            } else {
                // Terminate the linked actor
                if let Some(linked_actor_ref) = self.actors.get(&actor_pid) {
                    let mut linked_actor = linked_actor_ref.lock().unwrap();
                    if linked_actor.status != ActorStatus::Terminated {
                        linked_actor.status = ActorStatus::Terminated;
                    }

                    // Don't recursively call propagate_exit here to avoid stack overflow
                    // Instead, we'll handle it separately if needed
                }
            }
        }
        
        // Notify all monitors about the exit
        for actor_pid in monitoring_actors {
            // Send monitor exit message
            let monitor_exit_msg = Message::System(
                crate::mailbox::SystemMessage::MonitorExit(pid, reason.clone())
            );
            if let Some(target_actor_ref) = self.actors.get(&actor_pid) {
                let mut target_actor = target_actor_ref.lock().unwrap();
                // If actor was waiting, wake it up
                if target_actor.status == ActorStatus::Waiting {
                    target_actor.deliver_message(monitor_exit_msg);
                    self.run_queue.push_back(actor_pid);
                } else {
                    target_actor.mailbox.push(monitor_exit_msg);
                }
            }
        }
        
        // The original actor that died was already removed from active_actors counter
        // when it was marked as terminated in the main step loop.
        // If linked actors die as a result of propagation, they should be handled separately
        // by their own termination paths when they are scheduled again.
        // We don't decrement the counter here as it would be double-decrementing.
    }
}
