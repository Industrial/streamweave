//! Node discovery for distributed processing.

use std::net::SocketAddr;
use std::time::Duration;
use tokio::sync::RwLock;

/// Node discovery errors.
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
  /// Invalid address.
  #[error("Invalid address: {0}")]
  InvalidAddress(String),

  /// Discovery timeout.
  #[error("Discovery timeout")]
  Timeout,

  /// Network error.
  #[error("Network error: {0}")]
  Network(#[from] std::io::Error),

  /// Other discovery error.
  #[error("Discovery error: {0}")]
  Other(String),
}

/// Discovered node information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeInfo {
  /// Node address.
  pub address: SocketAddr,
  /// Node ID.
  pub node_id: String,
  /// Node capabilities.
  pub capabilities: Vec<String>,
  /// Last seen timestamp.
  pub last_seen: chrono::DateTime<chrono::Utc>,
}

/// Simple node discovery service.
pub struct NodeDiscovery {
  /// Known nodes.
  nodes: RwLock<Vec<NodeInfo>>,
  /// Discovery interval (stored for potential future use).
  #[allow(dead_code)]
  discovery_interval: Duration,
}

impl NodeDiscovery {
  /// Creates a new node discovery service.
  pub fn new(discovery_interval: Duration) -> Self {
    Self {
      nodes: RwLock::new(Vec::new()),
      discovery_interval,
    }
  }

  /// Registers a node.
  pub async fn register_node(&self, node: NodeInfo) {
    let mut nodes = self.nodes.write().await;
    // Update if exists, otherwise add
    if let Some(existing) = nodes.iter_mut().find(|n| n.address == node.address) {
      *existing = node;
    } else {
      nodes.push(node);
    }
  }

  /// Discovers nodes by attempting to connect to known addresses.
  pub async fn discover_nodes(
    &self,
    addresses: &[SocketAddr],
  ) -> Result<Vec<NodeInfo>, DiscoveryError> {
    let mut discovered = Vec::new();

    for &addr in addresses {
      // Try to connect with timeout
      match tokio::time::timeout(Duration::from_secs(5), tokio::net::TcpStream::connect(addr)).await
      {
        Ok(Ok(_stream)) => {
          // Connection successful - node is reachable
          discovered.push(NodeInfo {
            address: addr,
            node_id: format!("node-{}", addr),
            capabilities: vec![],
            last_seen: chrono::Utc::now(),
          });
        }
        _ => {
          // Connection failed - node not reachable
          continue;
        }
      }
    }

    Ok(discovered)
  }

  /// Gets all known nodes.
  pub async fn get_nodes(&self) -> Vec<NodeInfo> {
    self.nodes.read().await.clone()
  }

  /// Removes stale nodes that haven't been seen recently.
  pub async fn cleanup_stale(&self, max_age: Duration) {
    let now = chrono::Utc::now();
    let mut nodes = self.nodes.write().await;
    nodes.retain(|node| {
      let age = now.signed_duration_since(node.last_seen);
      age.to_std().is_ok_and(|d| d < max_age)
    });
  }
}

impl Default for NodeDiscovery {
  fn default() -> Self {
    Self::new(Duration::from_secs(30))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

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
}
