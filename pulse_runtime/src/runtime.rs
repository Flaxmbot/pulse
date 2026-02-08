use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::rc::Rc;

use pulse_core::{ActorId, PulseError, PulseResult, Value, Chunk, Constant};
use pulse_core::object::{Object, HeapInterface}; // Need HeapInterface to alloc string
use pulse_vm::{VM, VMStatus, CallFrame};
use crate::actor::{Actor, ActorStatus};
use crate::mailbox::Message;

pub struct Runtime {
    actors: HashMap<ActorId, Arc<Mutex<Actor>>>,
    run_queue: VecDeque<ActorId>,
    next_pid: u64,
    node_id: u128,
}

impl Runtime {
    pub fn new(node_id: u128) -> Self {
        Self {
            actors: HashMap::new(),
            run_queue: VecDeque::new(),
            next_pid: 1,
            node_id,
        }
    }

    pub fn spawn(&mut self, chunk: Chunk) -> ActorId {
        let pid = ActorId::new(self.node_id, self.next_pid);
        self.next_pid += 1;

        let vm = VM::new(chunk, pid);
        // VM::new initializes frame now.

        let actor = Actor::new(pid, vm);
        
        self.actors.insert(pid, Arc::new(Mutex::new(actor)));
        self.run_queue.push_back(pid);
        pid
    }

    pub fn spawn_with_rc(&mut self, chunk: Rc<Chunk>, ip: usize) -> ActorId {
        let pid = ActorId::new(self.node_id, self.next_pid);
        self.next_pid += 1;

        let vm = VM::new_spawn(chunk, pid, ip);

        let actor = Actor::new(pid, vm);
        
        self.actors.insert(pid, Arc::new(Mutex::new(actor)));
        self.run_queue.push_back(pid);
        pid
    }

    pub fn send(&mut self, target: ActorId, msg: Message) -> PulseResult<()> {
        if let Some(actor_ref) = self.actors.get(&target) {
            let mut actor = actor_ref.lock().unwrap();
            actor.mailbox.push(msg);
            // If actor was waiting, wake it up and schedule it
            if actor.status == ActorStatus::Waiting {
                actor.status = ActorStatus::Runnable;
                 // Note: In real system, we need to push to run_queue safely.
                 // Here we have &mut self, so it's fine.
                 self.run_queue.push_back(target);
            }
            Ok(())
        } else {
            Err(PulseError::ActorNotFound(target))
        }
    }

    // Step the scheduler: pick one actor, run slice, re-queue
    pub fn step(&mut self) -> bool {
        if let Some(pid) = self.run_queue.pop_front() {
             let actor_ref = match self.actors.get(&pid) {
                 Some(r) => r.clone(),
                 None => return true, 
             };

             // 1. Run VM slice and get status
             let status = {
                let mut actor = actor_ref.lock().unwrap();
                if actor.status == ActorStatus::Runnable {
                    actor.vm.run(100)
                } else {
                    VMStatus::Yielded // Should not happen if in run_queue
                }
             };

             // 2. Handle status (Effects)
             match status {
                 VMStatus::Running => {
                     self.run_queue.push_back(pid);
                 },
                 VMStatus::Yielded => {
                     // Explicit yield (not used yet, treat as Running)
                     self.run_queue.push_back(pid);
                 },
                 VMStatus::Blocked => {
                     // Receive: Check mailbox
                     let mut actor = actor_ref.lock().unwrap();
                     if let Some(msg) = actor.mailbox.pop() {
                          match msg {
                              Message::User(constant) => {
                                  // Convert Constant -> Value (allocating in destination VM heap)
                                  let val = match constant {
                                      Constant::Bool(b) => Value::Bool(b),
                                      Constant::Int(i) => Value::Int(i),
                                      Constant::Float(f) => Value::Float(f),
                                      Constant::Unit => Value::Unit,
                                      Constant::String(s) => {
                                          let handle = actor.vm.heap.alloc(Object::String(s));
                                          Value::Obj(handle)
                                      },
                                  };
                                  actor.vm.push(val);
                              },
                              _ => {},
                          }
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
                     let child_pid = self.spawn_with_rc(chunk_rc, offset);
                     
                     // Push child PID to parent
                     {
                         let mut actor = actor_ref.lock().unwrap();
                         actor.vm.push(Value::Pid(child_pid));
                     }
                     self.run_queue.push_back(pid);
                 },
                 VMStatus::Halted => {
                     let mut actor = actor_ref.lock().unwrap();
                     actor.status = ActorStatus::Terminated;
                     println!("Actor {:?} terminated", actor.id);
                 },
                 VMStatus::Error(e) => {
                     let mut actor = actor_ref.lock().unwrap();
                     actor.status = ActorStatus::Terminated;
                     println!("Actor {:?} crashed: {}", actor.id, e);
                 }
             }
             true
        } else {
            false // Idle
        }
    }
}
