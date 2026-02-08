use pulse_core::{Value, Constant, ActorId};
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub enum Message {
    User(Constant),
    System(SystemMessage),
}

#[derive(Debug, Clone)]
pub enum SystemMessage {
    Exit(ActorId, String),
    Link(ActorId),
    Monitor(ActorId),
}

pub struct Mailbox {
    queue: VecDeque<Message>,
}

impl Mailbox {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn push(&mut self, msg: Message) {
        self.queue.push_back(msg);
    }

    pub fn pop(&mut self) -> Option<Message> {
        self.queue.pop_front()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}
