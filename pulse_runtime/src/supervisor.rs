//! Supervision trees for Pulse actors
//!
//! Provides Erlang/OTP-style supervision with restart strategies:
//! - OneForOne: Restart only the failed child
//! - OneForAll: Restart all children if one fails
//! - RestForOne: Restart failed child and all children started after it

use crate::mailbox::Message;
use crate::runtime::RuntimeHandle;
use pulse_core::{ActorId, Constant};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Restart strategy for supervision trees
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RestartStrategy {
    /// Restart only the failed child
    OneForOne,
    /// Restart all children if one fails
    OneForAll,
    /// Restart failed child and all children started after it
    RestForOne,
}

/// Child specification for supervised actors
#[derive(Debug, Clone)]
pub struct ChildSpec {
    /// Unique child ID
    pub id: String,
    /// Function to start the child (returns closure constant)
    pub start: Constant,
    /// Arguments to pass to start function
    pub args: Vec<Constant>,
    /// Restart policy: permanent (always restart), transient (restart on error), temporary (never restart)
    pub restart: RestartPolicy,
    /// Maximum number of restarts within time period
    pub max_restarts: u32,
    /// Time period for max restarts (in seconds)
    pub restart_window: u64,
}

/// Restart policy for a child
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RestartPolicy {
    /// Always restart the child
    Permanent,
    /// Restart only if abnormal exit
    Transient,
    /// Never restart
    Temporary,
}

/// Information about a running child
#[derive(Debug)]
struct ChildInfo {
    spec: ChildSpec,
    pid: Option<ActorId>,
    restart_count: u32,
    last_restart: Option<Instant>,
    /// Index in start order (for RestForOne strategy)
    start_order: usize,
}

/// Supervisor actor state
pub struct Supervisor {
    runtime: RuntimeHandle,
    strategy: RestartStrategy,
    children: RwLock<HashMap<String, ChildInfo>>,
    next_order: RwLock<usize>,
}

impl Supervisor {
    /// Create a new supervisor
    pub fn new(runtime: RuntimeHandle, strategy: RestartStrategy) -> Self {
        Self {
            runtime,
            strategy,
            children: RwLock::new(HashMap::new()),
            next_order: RwLock::new(0),
        }
    }

    /// Start a child under supervision
    pub async fn start_child(&self, spec: ChildSpec) -> Result<ActorId, String> {
        let mut children = self.children.write().await;

        // Check if child with this ID already exists
        if children.contains_key(&spec.id) {
            return Err(format!("Child with ID '{}' already exists", spec.id));
        }

        // Start the child actor
        let pid = self.spawn_child(&spec).await?;

        // Get start order
        let mut order = self.next_order.write().await;
        let start_order = *order;
        *order += 1;

        // Track the child
        children.insert(
            spec.id.clone(),
            ChildInfo {
                spec,
                pid: Some(pid),
                restart_count: 0,
                last_restart: None,
                start_order,
            },
        );

        Ok(pid)
    }

    /// Terminate a child
    pub async fn terminate_child(&self, child_id: &str) -> Result<(), String> {
        let mut children = self.children.write().await;

        if let Some(info) = children.get_mut(child_id) {
            if let Some(pid) = info.pid {
                // Send exit signal to child
                let _ = self
                    .runtime
                    .send(
                        pid,
                        Message::System(crate::mailbox::SystemMessage::Exit(
                            ActorId::new(0, 0), // Supervisor PID placeholder
                            "shutdown".to_string(),
                        )),
                    )
                    .await;

                info.pid = None;
                Ok(())
            } else {
                Err(format!("Child '{}' is not running", child_id))
            }
        } else {
            Err(format!("Child '{}' not found", child_id))
        }
    }

    /// Handle child exit and apply restart strategy
    pub async fn handle_child_exit(&self, child_id: &str, reason: String) {
        let mut children = self.children.write().await;

        let should_restart = if let Some(info) = children.get(child_id) {
            match info.spec.restart {
                RestartPolicy::Permanent => true,
                RestartPolicy::Transient => reason != "normal",
                RestartPolicy::Temporary => false,
            }
        } else {
            return; // Child not found
        };

        if !should_restart {
            // Remove the child from supervision
            children.remove(child_id);
            return;
        }

        // Check restart limits
        let can_restart = if let Some(info) = children.get(child_id) {
            let now = Instant::now();
            let window = Duration::from_secs(info.spec.restart_window);

            if let Some(last_restart) = info.last_restart {
                if now - last_restart > window {
                    // Window passed, reset counter
                    true
                } else if info.restart_count >= info.spec.max_restarts {
                    // Too many restarts in window
                    false
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            false
        };

        if !can_restart {
            eprintln!(
                "[Supervisor] Child '{}' exceeded max restarts, removing from supervision",
                child_id
            );
            children.remove(child_id);
            return;
        }

        // Apply restart strategy
        let failed_order = children.get(child_id).map(|i| i.start_order).unwrap_or(0);
        let mut children_to_restart: Vec<(String, ChildSpec)> = Vec::new();

        match self.strategy {
            RestartStrategy::OneForOne => {
                // Restart only the failed child
                if let Some(info) = children.get(child_id) {
                    children_to_restart.push((child_id.to_string(), info.spec.clone()));
                }
            }
            RestartStrategy::OneForAll => {
                // Restart all children
                for (id, info) in children.iter() {
                    children_to_restart.push((id.clone(), info.spec.clone()));
                }
            }
            RestartStrategy::RestForOne => {
                // Restart failed child and all started after it
                for (id, info) in children.iter() {
                    if info.start_order >= failed_order {
                        children_to_restart.push((id.clone(), info.spec.clone()));
                    }
                }
            }
        }

        // Stop all children that need to be restarted
        for (id, _) in &children_to_restart {
            if let Some(info) = children.get(id) {
                if let Some(pid) = info.pid {
                    let _ = self
                        .runtime
                        .send(
                            pid,
                            Message::System(crate::mailbox::SystemMessage::Exit(
                                ActorId::new(0, 0),
                                "restart".to_string(),
                            )),
                        )
                        .await;
                }
            }
        }

        // Restart children in order
        for (id, spec) in children_to_restart {
            match self.spawn_child(&spec).await {
                Ok(new_pid) => {
                    if let Some(info) = children.get_mut(&id) {
                        info.pid = Some(new_pid);
                        info.restart_count += 1;
                        info.last_restart = Some(Instant::now());
                    }
                }
                Err(e) => {
                    eprintln!("[Supervisor] Failed to restart child '{}': {}", id, e);
                    children.remove(&id);
                }
            }
        }
    }

    /// Get list of running children
    pub async fn which_children(&self) -> Vec<(String, Option<ActorId>)> {
        let children = self.children.read().await;
        children
            .iter()
            .map(|(id, info)| (id.clone(), info.pid))
            .collect()
    }

    /// Count active children
    pub async fn active_children_count(&self) -> usize {
        let children = self.children.read().await;
        children.values().filter(|info| info.pid.is_some()).count()
    }

    /// Spawn a child actor
    async fn spawn_child(&self, spec: &ChildSpec) -> Result<ActorId, String> {
        let globals = std::collections::HashMap::new();
        let upvalues = Vec::new();

        let pid = self
            .runtime
            .spawn_from_actor(spec.start.clone(), upvalues, globals, spec.args.clone())
            .await;

        Ok(pid)
    }
}

/// Convenience function to create a supervisor and start it
pub async fn start_supervisor(
    runtime: RuntimeHandle,
    strategy: RestartStrategy,
    children: Vec<ChildSpec>,
) -> Result<ActorId, String> {
    let supervisor = Arc::new(Supervisor::new(runtime.clone(), strategy));

    // Start all children
    for spec in children {
        supervisor.start_child(spec).await?;
    }

    // Spawn the supervisor actor (in a real implementation, this would run the supervisor loop)
    // For now, we return a dummy PID
    Ok(ActorId::new(0, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_restart_policy() {
        assert!(matches!(RestartPolicy::Permanent, RestartPolicy::Permanent));
        assert!(matches!(
            RestartStrategy::OneForOne,
            RestartStrategy::OneForOne
        ));
    }
}
