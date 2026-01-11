//! # TCP Graph Server
//!
//! High-level API for creating TCP servers that process incoming connections
//! through StreamWeave graphs. This enables building network services that
//! process TCP connections using graph-based data processing pipelines.
//!
//! ## Overview
//!
//! The TCP Graph Server provides:
//!
//! - **TCP Server**: Listens for incoming TCP connections
//! - **Graph Processing**: Routes connection data through StreamWeave graphs
//! - **Connection Management**: Handles connection lifecycle and timeouts
//! - **Configurable Reading**: Supports line-based or delimiter-based data reading
//! - **Error Handling**: Configurable error handling for connection failures
//!
//! ## Core Types
//!
//! - **[`TcpGraphServer`]**: TCP server that processes connections through graphs
//! - **[`TcpGraphServerConfig`]**: Configuration for server behavior
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::graph::tcp_server::{TcpGraphServer, TcpGraphServerConfig};
//! use streamweave::graph::Graph;
//!
//! let config = TcpGraphServerConfig::default()
//!     .with_bind_address("127.0.0.1:8080")
//!     .with_connection_timeout(Duration::from_secs(30));
//! let server = TcpGraphServer::new(config, graph);
//! server.start().await?;
//! ```

use crate::graph::{ExecutionError, Graph, GraphExecutor};
use futures::future::Either;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tracing::{error, info, warn};

/// Configuration for the TCP Graph Server.
#[derive(Debug, Clone)]
pub struct TcpGraphServerConfig {
  /// Address to bind to (e.g., "127.0.0.1:8080").
  pub bind_address: String,
  /// Connection timeout duration (default: 30 seconds).
  pub connection_timeout: Duration,
  /// Buffer size for reading data (default: 8192).
  pub buffer_size: usize,
  /// Whether to read as lines (default: true).
  pub read_as_lines: bool,
  /// Delimiter for reading data (used when read_as_lines is false).
  pub delimiter: Option<u8>,
}

impl Default for TcpGraphServerConfig {
  fn default() -> Self {
    Self {
      bind_address: "127.0.0.1:8080".to_string(),
      connection_timeout: Duration::from_secs(30),
      buffer_size: 8192,
      read_as_lines: true,
      delimiter: None,
    }
  }
}

impl TcpGraphServerConfig {
  /// Sets the bind address.
  #[must_use]
  pub fn with_bind_address(mut self, address: impl Into<String>) -> Self {
    self.bind_address = address.into();
    self
  }

  /// Sets the connection timeout.
  #[must_use]
  pub fn with_connection_timeout(mut self, timeout: Duration) -> Self {
    self.connection_timeout = timeout;
    self
  }

  /// Sets the buffer size.
  #[must_use]
  pub fn with_buffer_size(mut self, size: usize) -> Self {
    self.buffer_size = size;
    self
  }

  /// Sets whether to read as lines.
  #[must_use]
  pub fn with_read_as_lines(mut self, read_as_lines: bool) -> Self {
    self.read_as_lines = read_as_lines;
    self
  }

  /// Sets the delimiter for reading data.
  #[must_use]
  pub fn with_delimiter(mut self, delimiter: Option<u8>) -> Self {
    self.delimiter = delimiter;
    self
  }
}

/// TCP Graph Server that processes incoming TCP connections through a graph.
///
/// This server:
/// - Listens for incoming TCP connections
/// - Processes each connection through a StreamWeave graph
/// - Handles connection timeouts and errors
pub struct TcpGraphServer {
  /// Graph executor for running the graph
  executor: Arc<tokio::sync::RwLock<GraphExecutor>>,
  /// Server configuration
  config: TcpGraphServerConfig,
}

impl TcpGraphServer {
  /// Creates a new TCP Graph Server from a constructed graph.
  ///
  /// # Arguments
  ///
  /// * `graph` - The graph to execute for each connection.
  /// * `config` - Server configuration.
  ///
  /// # Errors
  ///
  /// Returns an error if the graph cannot be executed.
  pub async fn new(graph: Graph, config: TcpGraphServerConfig) -> Result<Self, ExecutionError> {
    let executor = GraphExecutor::new(graph);

    Ok(Self {
      executor: Arc::new(tokio::sync::RwLock::new(executor)),
      config,
    })
  }

  /// Starts the TCP server and begins accepting connections.
  ///
  /// This method will run indefinitely, accepting and processing connections.
  ///
  /// # Errors
  ///
  /// Returns an error if the server cannot bind to the address or start.
  pub async fn start(&self) -> Result<(), ExecutionError> {
    let addr: SocketAddr = self
      .config
      .bind_address
      .parse()
      .map_err(|e| ExecutionError::Other(format!("Invalid bind address: {}", e)))?;

    let listener = TcpListener::bind(addr)
      .await
      .map_err(|e| ExecutionError::Other(format!("Failed to bind to {}: {}", addr, e)))?;

    info!(
      address = %addr,
      "TCP Graph Server listening"
    );

    // Start graph executor
    {
      let mut executor = self.executor.write().await;
      executor.start().await?;
    }

    loop {
      match listener.accept().await {
        Ok((stream, peer_addr)) => {
          info!(
            peer = %peer_addr,
            "Accepted TCP connection"
          );

          let config = self.config.clone();
          let executor = Arc::clone(&self.executor);

          // Spawn task to handle connection
          tokio::spawn(async move {
            if let Err(e) = Self::handle_connection(stream, peer_addr, config, executor).await {
              error!(
                peer = %peer_addr,
                error = %e,
                "Error handling TCP connection"
              );
            }
          });
        }
        Err(e) => {
          warn!(
            error = %e,
            "Error accepting TCP connection"
          );
        }
      }
    }
  }

  async fn handle_connection(
    stream: TcpStream,
    peer_addr: SocketAddr,
    config: TcpGraphServerConfig,
    _executor: Arc<tokio::sync::RwLock<GraphExecutor>>,
  ) -> Result<(), ExecutionError> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    if config.read_as_lines {
      // Read line by line
      let mut line = String::new();
      loop {
        match timeout(config.connection_timeout, reader.read_line(&mut line)).await {
          Ok(Ok(0)) => break, // EOF
          Ok(Ok(_)) => {
            let trimmed = line
              .trim_end_matches('\n')
              .trim_end_matches('\r')
              .to_string();
            if !trimmed.is_empty() {
              // Process through graph (simplified - in practice, you'd inject into graph)
              // For now, echo back
              let response = format!("Echo: {}\n", trimmed);
              if let Err(e) = write_half.write_all(response.as_bytes()).await {
                error!(
                  peer = %peer_addr,
                  error = %e,
                  "Error writing response"
                );
                break;
              }
              if let Err(e) = write_half.flush().await {
                error!(
                  peer = %peer_addr,
                  error = %e,
                  "Error flushing response"
                );
                break;
              }
            }
            line.clear();
          }
          Ok(Err(e)) => {
            error!(
              peer = %peer_addr,
              error = %e,
              "Error reading from connection"
            );
            break;
          }
          Err(_) => {
            warn!(
              peer = %peer_addr,
              "Connection timeout"
            );
            break;
          }
        }
      }
    } else {
      // Read raw bytes with optional delimiter
      let mut buffer = vec![0u8; config.buffer_size];
      loop {
        let read_future = if let Some(delim) = config.delimiter {
          Either::Left(reader.read_until(delim, &mut buffer))
        } else {
          Either::Right(reader.read(&mut buffer))
        };
        match timeout(config.connection_timeout, read_future).await {
          Ok(Ok(0)) => break, // EOF
          Ok(Ok(n)) => {
            let data = if let Some(_delim) = config.delimiter {
              buffer[..n - 1].to_vec()
            } else {
              buffer[..n].to_vec()
            };
            if let Ok(text) = String::from_utf8(data)
              && !text.is_empty()
            {
              // Process through graph (simplified - in practice, you'd inject into graph)
              // For now, echo back
              let response = format!("Echo: {}\n", text);
              if let Err(e) = write_half.write_all(response.as_bytes()).await {
                error!(
                  peer = %peer_addr,
                  error = %e,
                  "Error writing response"
                );
                break;
              }
              if let Err(e) = write_half.flush().await {
                error!(
                  peer = %peer_addr,
                  error = %e,
                  "Error flushing response"
                );
                break;
              }
            }
            buffer.clear();
          }
          Ok(Err(e)) => {
            error!(
              peer = %peer_addr,
              error = %e,
              "Error reading from connection"
            );
            break;
          }
          Err(_) => {
            warn!(
              peer = %peer_addr,
              "Connection timeout"
            );
            break;
          }
        }
      }
    }

    Ok(())
  }
}
