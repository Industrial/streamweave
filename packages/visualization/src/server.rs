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
