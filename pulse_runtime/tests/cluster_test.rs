//! Cluster tests for distributed Pulse
//! Tests node discovery and cluster management

#[tokio::test]
async fn test_cluster_creation() {
    // Test that we can create a cluster state
    let node_id = pulse_runtime::cluster::NodeId::new();
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let state = pulse_runtime::cluster::ClusterState::new(node_id.clone(), addr);
    
    tracing::info!("Created cluster state with node ID: {}", node_id.0);
    
    // Should have 1 member initially
    assert_eq!(state.member_count(), 1);
}

#[tokio::test]
async fn test_cluster_node_id() {
    let node_id = pulse_runtime::cluster::NodeId::new();
    assert!(!node_id.0.is_empty());
    tracing::info!("Node ID: {}", node_id.0);
}

#[tokio::test]
async fn test_cluster_members() {
    let node_id = pulse_runtime::cluster::NodeId::new();
    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let state = pulse_runtime::cluster::ClusterState::new(node_id, addr);
    
    // Get members - should just be ourselves
    let members = state.alive_nodes();
    assert_eq!(members.len(), 1);
}
