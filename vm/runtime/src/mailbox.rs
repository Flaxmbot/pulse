use pulse_ast::{ActorId, Constant};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    User(Constant),
    System(SystemMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemMessage {
    Exit(ActorId, String),
    MonitorExit(ActorId, String),
    Link(ActorId),
    Monitor(ActorId),
}

// Mailbox struct is removed in favor of tokio::sync::mpsc
// The Actor struct will hold the Receiver, and the Runtime/Handle will hold Senders.
