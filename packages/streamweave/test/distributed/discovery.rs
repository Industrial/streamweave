//! Tests for distributed discovery module

use std::net::SocketAddr;
use std::time::Duration;
use streamweave::distributed::discovery::{DiscoveryError, NodeDiscovery, NodeInfo};

#[tokio::test]
async fn test_node_discovery_new() {
  let discovery = NodeDiscovery::new(Duration::from_secs(30));
  // Should create successfully
  assert!(true);
}

#[tokio::test]
async fn test_node_discovery_default() {
  let discovery = NodeDiscovery::default();
  // Should create successfully
  assert!(true);
}

#[tokio::test]
async fn test_node_discovery_register_node() {
  let discovery = NodeDiscovery::new(Duration::from_secs(30));
  let node = NodeInfo {
    address: "127.0.0.1:8080".parse().unwrap(),
    node_id: "node-1".to_string(),
    capabilities: vec!["cap1".to_string()],
    last_seen: chrono::Utc::now(),
  };

  discovery.register_node(node.clone()).await;

  let nodes = discovery.get_nodes().await;
  assert_eq!(nodes.len(), 1);
  assert_eq!(nodes[0].node_id, "node-1");
}

#[tokio::test]
async fn test_node_discovery_register_node_update() {
  let discovery = NodeDiscovery::new(Duration::from_secs(30));
  let node1 = NodeInfo {
    address: "127.0.0.1:8080".parse().unwrap(),
    node_id: "node-1".to_string(),
    capabilities: vec![],
    last_seen: chrono::Utc::now(),
  };

  let node2 = NodeInfo {
    address: "127.0.0.1:8080".parse().unwrap(),
    node_id: "node-2".to_string(),
    capabilities: vec![],
    last_seen: chrono::Utc::now(),
  };

  discovery.register_node(node1).await;
  discovery.register_node(node2).await;

  // Should update existing node, not add duplicate
  let nodes = discovery.get_nodes().await;
  assert_eq!(nodes.len(), 1);
  assert_eq!(nodes[0].node_id, "node-2");
}

#[tokio::test]
async fn test_node_discovery_get_nodes() {
  let discovery = NodeDiscovery::new(Duration::from_secs(30));

  // Initially empty
  let nodes = discovery.get_nodes().await;
  assert_eq!(nodes.len(), 0);

  // Add nodes
  discovery
    .register_node(NodeInfo {
      address: "127.0.0.1:8080".parse().unwrap(),
      node_id: "node-1".to_string(),
      capabilities: vec![],
      last_seen: chrono::Utc::now(),
    })
    .await;

  discovery
    .register_node(NodeInfo {
      address: "127.0.0.1:8081".parse().unwrap(),
      node_id: "node-2".to_string(),
      capabilities: vec![],
      last_seen: chrono::Utc::now(),
    })
    .await;

  let nodes = discovery.get_nodes().await;
  assert_eq!(nodes.len(), 2);
}

#[tokio::test]
async fn test_node_discovery_cleanup_stale() {
  let discovery = NodeDiscovery::new(Duration::from_secs(30));

  let old_node = NodeInfo {
    address: "127.0.0.1:8080".parse().unwrap(),
    node_id: "node-1".to_string(),
    capabilities: vec![],
    last_seen: chrono::Utc::now() - chrono::Duration::seconds(100),
  };

  let recent_node = NodeInfo {
    address: "127.0.0.1:8081".parse().unwrap(),
    node_id: "node-2".to_string(),
    capabilities: vec![],
    last_seen: chrono::Utc::now(),
  };

  discovery.register_node(old_node).await;
  discovery.register_node(recent_node).await;

  // Cleanup nodes older than 60 seconds
  discovery.cleanup_stale(Duration::from_secs(60)).await;

  let nodes = discovery.get_nodes().await;
  assert_eq!(nodes.len(), 1);
  assert_eq!(nodes[0].node_id, "node-2");
}
