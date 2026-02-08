use serde::{Serialize, Deserialize};
use pulse_core::ActorId;
use crate::mailbox::Message;

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageEnvelope {
    pub target: ActorId,
    pub sender: Option<ActorId>,
    pub message: Message,
}

impl MessageEnvelope {
    pub fn new(target: ActorId, sender: Option<ActorId>, message: Message) -> Self {
        Self { target, sender, message }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}
