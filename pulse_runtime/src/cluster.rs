//! Cluster module for distributed Pulse
//! Provides node discovery, health monitoring, and cluster membership management
//! using UDP broadcast for discovery and heartbeat for failure detection

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use uuid::Uuid;

/// Default UDP port for cluster discovery
pub const DEFAULT_DISCOVERY_PORT: u16 = 5678;
/// Default heartbeat interval
pub const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(3);
/// Default node timeout (after this, node is considered dead)
pub const DEFAULT_NODE_TIMEOUT: Duration = Duration::from_secs(10);
/// Discovery broadcast address
pub const DISCOVERY_ADDRESS: &str = "255.255.255.255:5678";

/// Unique node identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a node in the cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub address: SocketAddr,
    /// Last heartbeat as Unix timestamp (milliseconds)
    pub last_heartbeat_ms: u64,
    pub is_alive: bool,
}

impl Node {
    pub fn new(id: NodeId, address: SocketAddr) -> Self {
        Self {
            id,
            address,
            last_heartbeat_ms: current_time_ms(),
            is_alive: true,
        }
    }

    pub fn new_from_network(id: NodeId, address: SocketAddr) -> Self {
        Self {
            id,
            address,
            last_heartbeat_ms: current_time_ms(),
            is_alive: true,
        }
    }

    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat_ms = current_time_ms();
        self.is_alive = true;
    }

    pub fn is_timed_out(&self, timeout: Duration) -> bool {
        let now_ms = current_time_ms();
        now_ms.saturating_sub(self.last_heartbeat_ms) > timeout.as_millis() as u64
    }
}

/// Get current time in milliseconds since Unix epoch
fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Cluster message types for UDP communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusterMessage {
    /// Announce presence (broadcast by new nodes)
    Announce {
        node_id: NodeId,
        address: SocketAddr,
    },
    /// Heartbeat to keep node alive
    Heartbeat {
        node_id: NodeId,
        address: SocketAddr,
    },
    /// Request full member list
    GossipRequest {
        node_id: NodeId,
    },
    /// Response with member list
    GossipResponse {
        node_id: NodeId,
        members: Vec<Node>,
    },
    /// Notify node is leaving
    Leave {
        node_id: NodeId,
    },
}

/// Cluster state management
pub struct ClusterState {
    /// Our node ID
    pub node_id: NodeId,
    /// Our socket address
    pub address: SocketAddr,
    /// All known nodes (including ourselves)
    pub nodes: HashMap<String, Node>,
    /// Channel to notify of membership changes
    membership_tx: mpsc::UnboundedSender<MembershipEvent>,
}

/// Membership events for external notification
#[derive(Debug, Clone)]
pub enum MembershipEvent {
    NodeJoined(NodeId, SocketAddr),
    NodeLeft(NodeId),
    NodeFailed(NodeId),
}

impl ClusterState {
    pub fn new(node_id: NodeId, address: SocketAddr) -> Self {
        let (membership_tx, _) = mpsc::unbounded_channel();
        
        let mut nodes = HashMap::new();
        // Add ourselves to the cluster
        nodes.insert(node_id.0.clone(), Node::new(node_id.clone(), address));
        
        Self {
            node_id,
            address,
            nodes,
            membership_tx,
        }
    }

    /// Add or update a node in the cluster
    pub fn upsert_node(&mut self, node: Node) -> Option<MembershipEvent> {
        let node_id = node.id.0.clone();
        let is_new = !self.nodes.contains_key(&node_id);
        let address = node.address;
        
        if is_new {
            self.nodes.insert(node_id.clone(), node);
            Some(MembershipEvent::NodeJoined(NodeId(node_id), address))
        } else {
            if let Some(existing) = self.nodes.get_mut(&node_id) {
                existing.update_heartbeat();
            }
            None
        }
    }

    /// Remove a node from the cluster
    pub fn remove_node(&mut self, node_id: &NodeId) -> Option<MembershipEvent> {
        if self.nodes.remove(&node_id.0).is_some() {
            Some(MembershipEvent::NodeLeft(node_id.clone()))
        } else {
            None
        }
    }

    /// Get all alive nodes
    pub fn alive_nodes(&self) -> Vec<Node> {
        self.nodes.values()
            .filter(|n| n.is_alive)
            .cloned()
            .collect()
    }

    /// Check for timed out nodes
    pub fn check_timeouts(&mut self, timeout: Duration) -> Vec<MembershipEvent> {
        let mut events = Vec::new();
        
        let timed_out: Vec<String> = self.nodes.iter()
            .filter(|(id, node)| {
                id.as_str() != self.node_id.0 && node.is_timed_out(timeout)
            })
            .map(|(id, _)| id.clone())
            .collect();
        
        for id in timed_out {
            if let Some(node) = self.nodes.get(&id) {
                events.push(MembershipEvent::NodeFailed(node.id.clone()));
                self.nodes.remove(&id);
            }
        }
        
        events
    }

    /// Get member count
    pub fn member_count(&self) -> usize {
        self.nodes.len()
    }
}

/// Cluster manager for handling cluster operations
pub struct Cluster {
    state: Arc<RwLock<ClusterState>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl Cluster {
    /// Create a new cluster (standalone node)
    pub async fn new(port: u16) -> std::io::Result<Self> {
        let node_id = NodeId::new();
        let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
        
        let state = Arc::new(RwLock::new(ClusterState::new(node_id, addr)));
        
        Ok(Self {
            state,
            shutdown_tx: None,
        })
    }

    /// Start the cluster with UDP discovery
    pub async fn start(&mut self) -> std::io::Result<()> {
        let state = self.state.clone();
        
        // Create UDP socket for discovery
        let socket = UdpSocket::bind(DISCOVERY_ADDRESS).await?;
        socket.set_broadcast(true)?;
        
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);
        
        // Start discovery task
        tokio::spawn(async move {
            Self::discovery_loop(socket, state, &mut shutdown_rx).await;
        });
        
        // Start heartbeat task
        let heartbeat_state = self.state.clone();
        let heartbeat_socket = UdpSocket::bind("0.0.0.0:0").await?;
        heartbeat_socket.set_broadcast(true)?;
        
        tokio::spawn(async move {
            Self::heartbeat_loop(heartbeat_socket, heartbeat_state).await;
        });
        
        // Start timeout checker
        let timeout_state = self.state.clone();
        tokio::spawn(async move {
            Self::timeout_checker_loop(timeout_state).await;
        });
        
        tracing::info!("Cluster started with node ID: {:?}", 
            self.state.read().await.node_id);
        
        Ok(())
    }

    /// Discovery loop - listens for UDP messages
    async fn discovery_loop(
        socket: UdpSocket,
        state: Arc<RwLock<ClusterState>>,
        shutdown_rx: &mut mpsc::Receiver<()>,
    ) {
        let mut buf = [0u8; 4096];
        let mut shutdown = shutdown_rx;
        
        loop {
            tokio::select! {
                result = socket.recv_from(&mut buf) => {
                    match result {
                        Ok((len, addr)) => {
                            if let Ok(msg) = serde_json::from_slice::<ClusterMessage>(&buf[..len]) {
                                Self::handle_message(msg, addr, &state).await;
                            }
                        }
                        Err(e) => {
                            tracing::error!("Discovery recv error: {}", e);
                            break;
                        }
                    }
                }
                _ = shutdown.recv() => {
                    tracing::info!("Discovery loop shutting down");
                    break;
                }
            }
        }
    }

    /// Handle incoming cluster messages
    async fn handle_message(
        msg: ClusterMessage,
        _addr: SocketAddr,
        state: &Arc<RwLock<ClusterState>>,
    ) {
        match msg {
            ClusterMessage::Announce { node_id, address } => {
                if node_id.0 != state.read().await.node_id.0 {
                    let node = Node::new(node_id.clone(), address);
                    let mut s = state.write().await;
                    if let Some(event) = s.upsert_node(node) {
                        tracing::info!("Node joined: {:?}", event);
                    }
                }
            }
            
            ClusterMessage::Heartbeat { node_id, address } => {
                if node_id.0 != state.read().await.node_id.0 {
                    let mut node = Node::new(node_id.clone(), address);
                    node.update_heartbeat();
                    let mut s = state.write().await;
                    s.upsert_node(node);
                }
            }
            
            ClusterMessage::GossipRequest { node_id } => {
                if node_id.0 != state.read().await.node_id.0 {
                    let members = state.read().await.alive_nodes();
                    let response = ClusterMessage::GossipResponse {
                        node_id: state.read().await.node_id.clone(),
                        members,
                    };
                    // In a real implementation, we'd send this back to the requester
                    tracing::debug!("Received gossip request from {:?}", node_id);
                }
            }
            
            ClusterMessage::GossipResponse { node_id, members } => {
                if node_id.0 != state.read().await.node_id.0 {
                    let mut s = state.write().await;
                    for member in members {
                        if member.id.0 != s.node_id.0 {
                            s.upsert_node(member);
                        }
                    }
                }
            }
            
            ClusterMessage::Leave { node_id } => {
                let mut s = state.write().await;
                if let Some(event) = s.remove_node(&node_id) {
                    tracing::info!("Node left: {:?}", event);
                }
            }
        }
    }

    /// Heartbeat loop - broadcasts our presence
    async fn heartbeat_loop(socket: UdpSocket, state: Arc<RwLock<ClusterState>>) {
        let mut ticker = interval(DEFAULT_HEARTBEAT_INTERVAL);
        
        loop {
            ticker.tick().await;
            
            let (node_id, addr) = {
                let s = state.read().await;
                (s.node_id.clone(), s.address)
            };
            
            let msg = ClusterMessage::Heartbeat {
                node_id: node_id.clone(),
                address: addr,
            };
            
            if let Ok(bytes) = serde_json::to_vec(&msg) {
                let _ = socket.send_to(&bytes, DISCOVERY_ADDRESS).await;
            }
        }
    }

    /// Timeout checker - detects failed nodes
    async fn timeout_checker_loop(state: Arc<RwLock<ClusterState>>) {
        let mut ticker = interval(Duration::from_secs(1));
        
        loop {
            ticker.tick().await;
            
            let mut s = state.write().await;
            let events = s.check_timeouts(DEFAULT_NODE_TIMEOUT);
            
            for event in events {
                tracing::warn!("Node failed: {:?}", event);
            }
        }
    }

    /// Join an existing cluster by contacting a known node
    pub async fn join(&self, known_address: SocketAddr) -> std::io::Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        
        let (node_id, addr) = {
            let s = self.state.read().await;
            (s.node_id.clone(), s.address)
        };
        
        // Send announce message to known node
        let msg = ClusterMessage::Announce {
            node_id: node_id.clone(),
            address: addr,
        };
        
        if let Ok(bytes) = serde_json::to_vec(&msg) {
            socket.send_to(&bytes, known_address).await?;
            tracing::info!("Sent join request to {}", known_address);
        }
        
        // Also broadcast so others can discover us
        let broadcast_msg = ClusterMessage::Announce {
            node_id,
            address: addr,
        };
        
        if let Ok(bytes) = serde_json::to_vec(&broadcast_msg) {
            let _ = socket.send_to(&bytes, DISCOVERY_ADDRESS).await;
        }
        
        Ok(())
    }

    /// Leave the cluster
    pub async fn leave(&self) -> std::io::Result<()> {
        let (node_id, addr) = {
            let s = self.state.read().await;
            (s.node_id.clone(), s.address)
        };
        
        // Broadcast leave message
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        let msg = ClusterMessage::Leave {
            node_id: node_id.clone(),
        };
        
        if let Ok(bytes) = serde_json::to_vec(&msg) {
            let _ = socket.send_to(&bytes, DISCOVERY_ADDRESS).await;
        }
        
        tracing::info!("Node {} left the cluster", node_id.0);
        
        // Send shutdown signal
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(()).await;
        }
        
        Ok(())
    }

    /// Get current node ID
    pub async fn node_id(&self) -> NodeId {
        self.state.read().await.node_id.clone()
    }

    /// Get all cluster members
    pub async fn members(&self) -> Vec<Node> {
        self.state.read().await.alive_nodes()
    }

    /// Get member count
    pub async fn member_count(&self) -> usize {
        self.state.read().await.member_count()
    }

    /// Check if cluster has other nodes
    pub async fn is_clustered(&self) -> bool {
        self.state.read().await.member_count() > 1
    }
}

/// Create a new cluster with auto-generated node ID
pub async fn create_cluster(port: u16) -> std::io::Result<Cluster> {
    let mut cluster = Cluster::new(port).await?;
    cluster.start().await?;
    Ok(cluster)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use std::time::Duration;

    #[tokio::test]
    async fn test_node_id_generation() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        assert_ne!(id1.0, id2.0);
    }

    #[tokio::test]
    async fn test_cluster_state() {
        let node_id = NodeId::new();
        let addr: SocketAddr = (Ipv4Addr::new(127, 0, 0, 1), 8080).into();
        
        let mut state = ClusterState::new(node_id.clone(), addr);
        
        // Should have 1 member (ourselves)
        assert_eq!(state.member_count(), 1);
        
        // Add another node
        let other_node = Node::new(NodeId::new(), (Ipv4Addr::new(127, 0, 0, 2), 8081).into());
        let event = state.upsert_node(other_node.clone());
        assert!(event.is_some());
        assert_eq!(state.member_count(), 2);
        
        // Check alive nodes
        let alive = state.alive_nodes();
        assert_eq!(alive.len(), 2);
    }

    #[tokio::test]
    async fn test_timeout_detection() {
        let node_id = NodeId::new();
        let addr: SocketAddr = (Ipv4Addr::new(127, 0, 0, 1), 8080).into();
        
        let mut state = ClusterState::new(node_id, addr);
        
        // Add a "stale" node with old timestamp
        let mut stale_node = Node::new(NodeId::new(), (Ipv4Addr::new(127, 0, 0, 2), 8081).into());
        stale_node.last_heartbeat_ms = 0; // Very old timestamp
        state.upsert_node(stale_node);
        
        // Check timeouts
        let events = state.check_timeouts(Duration::from_secs(10));
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn test_cluster_lifecycle() {
        // Create cluster on random port - this tests basic state management
        // Note: UDP broadcast may not work in all test environments
        let node_id = NodeId::new();
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let state = ClusterState::new(node_id.clone(), addr);
        
        // Should have 1 member
        assert_eq!(state.member_count(), 1);
        
        tracing::info!("Created cluster state with node ID: {}", node_id.0);
    }
}
