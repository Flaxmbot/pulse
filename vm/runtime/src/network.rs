use crate::mailbox::Message;
use pulse_ast::object::Function;
use pulse_ast::ActorId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    pub target: ActorId,
    pub sender: Option<ActorId>,
    pub message: Message,
}

impl MessageEnvelope {
    pub fn new(target: ActorId, sender: Option<ActorId>, message: Message) -> Self {
        Self {
            target,
            sender,
            message,
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}

/// Remote actor spawn request - sent when spawning an actor on a remote node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSpawnRequest {
    /// The function/closure to spawn
    pub function: Arc<Function>,
    /// Arguments to pass to the actor (serializable values only)
    pub args: Vec<pulse_ast::Value>,
    /// Name of the actor for debugging
    pub name: Option<String>,
}

impl RemoteSpawnRequest {
    pub fn new(function: Arc<Function>, args: Vec<pulse_ast::Value>, name: Option<String>) -> Self {
        Self {
            function,
            args,
            name,
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}

/// Remote actor spawn response - sent back after spawning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSpawnResponse {
    /// The ActorId of the newly spawned actor
    pub actor_id: ActorId,
    /// Whether the spawn was successful
    pub success: bool,
    /// Error message if spawn failed
    pub error: Option<String>,
}

impl RemoteSpawnResponse {
    pub fn success(actor_id: ActorId) -> Self {
        Self {
            actor_id,
            success: true,
            error: None,
        }
    }

    pub fn failure(actor_id: ActorId, error: String) -> Self {
        Self {
            actor_id,
            success: false,
            error: Some(error),
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}
