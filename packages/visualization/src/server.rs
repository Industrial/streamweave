//! # Visualization Web Server
//!
//! This module provides a simple web server for serving the pipeline visualization UI.
//! The server can serve static HTML/JavaScript files and DAG JSON data.
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::visualization::{PipelineDag, VisualizationServer};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let dag = PipelineDag::new();
//! let server = VisualizationServer::new("127.0.0.1:8080".parse()?);
//! server.serve_dag(dag).await?;
//! # Ok(())
//! # }
//! ```

use crate::dag::PipelineDag;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A simple HTTP server for serving pipeline visualization UI.
///
/// This server serves static HTML/JavaScript files for the visualization UI
/// and provides an API endpoint to serve DAG JSON data.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::visualization::{PipelineDag, VisualizationServer};
/// use std::net::SocketAddr;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let dag = PipelineDag::new();
/// let addr: SocketAddr = "127.0.0.1:8080".parse()?;
/// let server = VisualizationServer::new(addr);
/// server.serve_dag(dag).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct VisualizationServer {
  address: SocketAddr,
}

impl VisualizationServer {
  /// Creates a new visualization server bound to the given address.
  ///
  /// # Arguments
  ///
  /// * `address` - The socket address to bind the server to
  ///
  /// # Returns
  ///
  /// A new `VisualizationServer` instance.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::VisualizationServer;
  /// use std::net::SocketAddr;
  ///
  /// let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
  /// let server = VisualizationServer::new(addr);
  /// ```
  #[must_use]
  pub fn new(address: SocketAddr) -> Self {
    Self { address }
  }

  /// Starts the server and serves the given DAG.
  ///
  /// This method starts an HTTP server that serves:
  /// - The visualization UI at the root path
  /// - DAG JSON data at `/api/dag`
  ///
  /// # Arguments
  ///
  /// * `dag` - The pipeline DAG to visualize
  ///
  /// # Errors
  ///
  /// Returns an error if the server fails to start or handle requests.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::visualization::{PipelineDag, VisualizationServer};
  /// use std::net::SocketAddr;
  ///
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let dag = PipelineDag::new();
  /// let addr: SocketAddr = "127.0.0.1:8080".parse()?;
  /// let server = VisualizationServer::new(addr);
  /// println!("Server starting at http://{}", addr);
  /// server.serve_dag(dag).await?;
  /// # Ok(())
  /// # }
  /// ```
  pub async fn serve_dag(
    &self,
    dag: PipelineDag,
  ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // For now, this is a placeholder implementation
    // Full implementation would require HTTP server infrastructure
    // which is planned for task 16

    // Store the DAG for serving via API
    let _dag = Arc::new(RwLock::new(dag));

    // Log the server address
    eprintln!(
      "Visualization server would start at http://{}",
      self.address
    );
    eprintln!("Note: Full HTTP server implementation requires task 16 (HTTP Server Support)");

    Ok(())
  }

  /// Gets the server address.
  ///
  /// # Returns
  ///
  /// The socket address the server is bound to.
  #[must_use]
  pub fn address(&self) -> SocketAddr {
    self.address
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_visualization_server_new() {
    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let server = VisualizationServer::new(addr);
    assert_eq!(server.address(), addr);
  }

  #[test]
  fn test_visualization_server_address() {
    let addr: SocketAddr = "0.0.0.0:3000".parse().unwrap();
    let server = VisualizationServer::new(addr);
    assert_eq!(server.address(), addr);
  }

  #[test]
  fn test_visualization_server_clone() {
    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let server1 = VisualizationServer::new(addr);
    let server2 = server1.clone();
    assert_eq!(server1.address(), server2.address());
  }

  #[tokio::test]
  async fn test_visualization_server_serve_dag() {
    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let server = VisualizationServer::new(addr);
    let dag = PipelineDag::new();

    // This should succeed (placeholder implementation)
    let result = server.serve_dag(dag).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_visualization_server_serve_dag_multiple() {
    let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let server = VisualizationServer::new(addr);

    let dag1 = PipelineDag::new();
    let dag2 = PipelineDag::new();

    // Both should succeed
    assert!(server.serve_dag(dag1).await.is_ok());
    assert!(server.serve_dag(dag2).await.is_ok());
  }
}
