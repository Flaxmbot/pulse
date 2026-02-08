use pulse_core::{ActorId, Value, Constant};
use pulse_core::object::{Object, Closure, Function};
use pulse_vm::VM;
use crate::mailbox::{Mailbox, Message, SystemMessage};
use std::collections::HashSet;

#[derive(Debug, PartialEq)]
pub enum ActorStatus {
    Starting,
    Runnable,
    Waiting, // Waiting for message
    Terminated,
}

pub struct Actor {
    pub id: ActorId,
    pub mailbox: Mailbox,
    pub vm: VM,
    pub status: ActorStatus,
    pub links: HashSet<ActorId>,
    pub monitors: HashSet<ActorId>,
    pub trapping_exits: bool,
}

impl Actor {
    pub fn new(id: ActorId, vm: VM) -> Self {
        Self {
            id,
            mailbox: Mailbox::new(),
            vm,
            status: ActorStatus::Runnable,
            links: HashSet::new(),
            monitors: HashSet::new(),
            trapping_exits: false,
        }
    }

    pub fn is_runnable(&self) -> bool {
        self.status == ActorStatus::Runnable
    }

    pub fn deliver_message(&mut self, msg: Message) {
        match msg {
            Message::User(constant) => {
                let val = match constant {
                    Constant::Bool(b) => Value::Bool(b),
                    Constant::Int(i) => Value::Int(i),
                    Constant::Float(f) => Value::Float(f),
                    Constant::Unit => Value::Unit,
                    Constant::String(s) => {
                        let handle = self.vm.heap.alloc(Object::String(s));
                        Value::Obj(handle)
                    },
                    Constant::Function(f) => {
                        let closure = Closure {
                            function: *f,
                            upvalues: Vec::new(),
                        };
                        let handle = self.vm.heap.alloc(Object::Closure(closure));
                        Value::Obj(handle)
                    }
                };
                self.vm.push(val);
            },
            Message::System(sys_msg) => {
                match sys_msg {
                    SystemMessage::MonitorExit(pid, reason) => {
                        let down_str = self.vm.heap.alloc(Object::String("DOWN".to_string()));
                        let reason_str = self.vm.heap.alloc(Object::String(reason));
                        let pid_val = Value::Pid(pid);
                        
                        let list_handle = self.vm.heap.alloc(Object::List(vec![
                            Value::Obj(down_str),
                            pid_val,
                            Value::Obj(reason_str)
                        ]));
                        self.vm.push(Value::Obj(list_handle));
                    },
                    SystemMessage::Exit(pid, reason) => {
                        let exit_str = self.vm.heap.alloc(Object::String("EXIT".to_string()));
                        let reason_str = self.vm.heap.alloc(Object::String(reason));
                        let pid_val = Value::Pid(pid);
                        
                        let list_handle = self.vm.heap.alloc(Object::List(vec![
                            Value::Obj(exit_str),
                            pid_val,
                            Value::Obj(reason_str)
                        ]));
                        self.vm.push(Value::Obj(list_handle));
                    },
                    _ => {
                        // Ignore other system messages or push Unit?
                        // If receive expects a value, we must push something or fail?
                        // For now push Unit to avoid stack underflow if they try to pop
                        self.vm.push(Value::Unit);
                    }
                }
            }
        }
        self.status = ActorStatus::Runnable;
    }
}
