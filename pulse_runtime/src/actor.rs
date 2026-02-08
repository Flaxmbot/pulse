use pulse_core::ActorId;
use pulse_vm::VM;
use crate::mailbox::Mailbox;

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
}

impl Actor {
    pub fn new(id: ActorId, vm: VM) -> Self {
        Self {
            id,
            mailbox: Mailbox::new(),
            vm,
            status: ActorStatus::Runnable,
        }
    }

    pub fn is_runnable(&self) -> bool {
        self.status == ActorStatus::Runnable
    }
}
