use proptest::prelude::*;
use std::net::SocketAddr;
use std::time::Duration;
use streamweave_distributed::{DiscoveryError, NodeDiscovery, NodeInfo};

async fn test_node_discovery_register_async(node_id: String, capabilities: Vec<String>) {
  let discovery = NodeDiscovery::default();
  let address: SocketAddr = "127.0.0.1:8080".parse().unwrap();
  let node = NodeInfo {
    address,
    node_id: node_id.clone(),
    capabilities: capabilities.clone(),
    last_seen: chrono::Utc::now(),
  };

  discovery.register_node(node).await;
  let nodes = discovery.get_nodes().await;
  assert_eq!(nodes.len(), 1);
  assert_eq!(nodes[0].node_id, node_id);
  assert_eq!(nodes[0].capabilities, capabilities);
}

proptest! {
  #[test]
  fn test_node_discovery_register(
    node_id in prop::string::string_regex("[a-zA-Z0-9_-]+").unwrap(),
    capabilities in prop::collection::vec(
      prop::string::string_regex("[a-zA-Z0-9_-]+").unwrap(),
      0..10
    ),
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_node_discovery_register_async(node_id, capabilities));
  }
}

#[tokio::test]
async fn test_node_discovery_register_update() {
  let discovery = NodeDiscovery::default();
  let address: SocketAddr = "127.0.0.1:8080".parse().unwrap();

  // Register first node
  let node1 = NodeInfo {
    address,
    node_id: "node-1".to_string(),
    capabilities: vec!["cap1".to_string()],
    last_seen: chrono::Utc::now(),
  };
  discovery.register_node(node1).await;

  // Register same address with different node_id (should update)
  let node2 = NodeInfo {
    address,
    node_id: "node-2".to_string(),
    capabilities: vec!["cap2".to_string()],
    last_seen: chrono::Utc::now(),
  };
  discovery.register_node(node2).await;

  let nodes = discovery.get_nodes().await;
  assert_eq!(nodes.len(), 1); // Should still be one node (updated)
  assert_eq!(nodes[0].node_id, "node-2");
}

#[tokio::test]
async fn test_node_discovery_discover_nodes() {
  let discovery = NodeDiscovery::new(Duration::from_secs(30));

  // Start a TCP listener
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
  let addr = listener.local_addr().unwrap();

  let _server_handle = tokio::spawn(async move {
    let (_stream, _) = listener.accept().await.unwrap();
  });

  // Discover nodes
  let addresses = vec![addr];
  let discovered = discovery.discover_nodes(&addresses).await.unwrap();

  // Should discover the listening node
  assert!(!discovered.is_empty());
  assert_eq!(discovered[0].address, addr);
}

#[tokio::test]
async fn test_node_discovery_discover_nodes_unreachable() {
  let discovery = NodeDiscovery::new(Duration::from_secs(30));

  // Try to discover unreachable address
  let unreachable: SocketAddr = "127.0.0.1:1".parse().unwrap();
  let addresses = vec![unreachable];
  let discovered = discovery.discover_nodes(&addresses).await.unwrap();

  // Should return empty (unreachable nodes are skipped)
  assert!(discovered.is_empty());
}

#[tokio::test]
async fn test_node_discovery_cleanup_stale() {
  let discovery = NodeDiscovery::new(Duration::from_secs(30));
  let address: SocketAddr = "127.0.0.1:8080".parse().unwrap();

  // Register a stale node (old timestamp)
  let stale_node = NodeInfo {
    address,
    node_id: "stale-node".to_string(),
    capabilities: vec![],
    last_seen: chrono::Utc::now() - chrono::Duration::seconds(100),
  };
  discovery.register_node(stale_node).await;

  // Register a fresh node
  let fresh_node = NodeInfo {
    address: "127.0.0.1:8081".parse().unwrap(),
    node_id: "fresh-node".to_string(),
    capabilities: vec![],
    last_seen: chrono::Utc::now(),
  };
  discovery.register_node(fresh_node).await;

  // Cleanup nodes older than 50 seconds
  discovery.cleanup_stale(Duration::from_secs(50)).await;

  let nodes = discovery.get_nodes().await;
  // Only fresh node should remain
  assert_eq!(nodes.len(), 1);
  assert_eq!(nodes[0].node_id, "fresh-node");
}

#[test]
fn test_discovery_error_display() {
  let error = DiscoveryError::InvalidAddress("test".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Invalid address"));
  assert!(error_str.contains("test"));
}

#[test]
fn test_discovery_error_timeout() {
  let error = DiscoveryError::Timeout;
  let error_str = format!("{}", error);
  assert!(error_str.contains("Discovery timeout"));
}

#[test]
fn test_discovery_error_other() {
  let error = DiscoveryError::Other("test".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Discovery error"));
  assert!(error_str.contains("test"));
}

#[test]
fn test_discovery_error_debug() {
  let error = DiscoveryError::Timeout;
  let debug_str = format!("{:?}", error);
  assert!(!debug_str.is_empty());
}

#[test]
fn test_discovery_error_is_error() {
  let error = DiscoveryError::Timeout;
  use std::error::Error;
  assert!(!error.source().is_some());
}

#[test]
fn test_node_info_equality() {
  let node1 = NodeInfo {
    address: "127.0.0.1:8080".parse().unwrap(),
    node_id: "node-1".to_string(),
    capabilities: vec!["cap1".to_string()],
    last_seen: chrono::Utc::now(),
  };

  let node2 = NodeInfo {
    address: "127.0.0.1:8080".parse().unwrap(),
    node_id: "node-1".to_string(),
    capabilities: vec!["cap1".to_string()],
    last_seen: chrono::Utc::now(),
  };

  assert_eq!(node1, node2);
}

#[test]
fn test_node_info_debug() {
  let node = NodeInfo {
    address: "127.0.0.1:8080".parse().unwrap(),
    node_id: "node-1".to_string(),
    capabilities: vec![],
    last_seen: chrono::Utc::now(),
  };

  let debug_str = format!("{:?}", node);
  assert!(!debug_str.is_empty());
  assert!(debug_str.contains("node-1"));
}
