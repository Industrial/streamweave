//! TCP receive node for receiving data from TCP connections in graphs.
//!
//! This module provides [`TcpReceive`], a graph node that receives data from
//! TCP connections. It wraps [`TcpReceiveTransformer`] for use in StreamWeave
//! graphs. It connects to a remote TCP address and receives data, making it
//! ideal for network-based data ingestion in graph-based pipelines.
//!
//! # Overview
//!
//! [`TcpReceive`] is useful for receiving data from TCP connections in
//! graph-based pipelines. It connects to a remote TCP address and receives
//! data, supporting configurable connection options, buffer sizes, and reading
//! modes (lines or delimiter-based).
//!
//! # Key Concepts
//!
//! - **TCP Reception**: Receives data from TCP connections
//! - **Configurable Options**: Supports timeout, buffer size, reading modes
//! - **Line-Based or Delimiter-Based**: Supports reading as lines or with custom delimiters
//! - **Transformer Wrapper**: Wraps `TcpReceiveTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`TcpReceive`]**: Node that receives data from TCP connections
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::TcpReceive;
//!
//! // Create a TCP receive node
//! let tcp_receive = TcpReceive::new("127.0.0.1:8080");
//! ```
//!
//! ## With Configuration
//!
//! ```rust
//! use streamweave::graph::nodes::TcpReceive;
//!
//! // Create a TCP receive node with configuration
//! let tcp_receive = TcpReceive::new("127.0.0.1:8080")
//!     .with_timeout_secs(10)
//!     .with_buffer_size(8192)
//!     .with_read_as_lines(true)
//!     .with_max_items(Some(1000));
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::TcpReceive;
//! use streamweave::ErrorStrategy;
//!
//! // Create a TCP receive node with error handling
//! let tcp_receive = TcpReceive::new("127.0.0.1:8080")
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("tcp-receiver".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **TCP Connection**: Uses Tokio's async TCP for non-blocking network I/O
//! - **Configurable Options**: Supports timeout, buffer size, reading modes for
//!   flexibility
//! - **Line/Delimiter-Based**: Supports both line-based and delimiter-based reading
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`TcpReceive`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::TcpReceiveTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that receives data from TCP connections.
///
/// This node wraps `TcpReceiveTransformer` for use in graphs.
/// Useful for graph composition when you need to receive TCP data
/// in the middle of a graph.
pub struct TcpReceive {
  /// The underlying receive transformer
  transformer: TcpReceiveTransformer,
}

impl TcpReceive {
  /// Creates a new `TcpReceive` node.
  ///
  /// # Arguments
  ///
  /// * `address` - The remote address to connect to (e.g., "127.0.0.1:8080").
  pub fn new(address: impl Into<String>) -> Self {
    Self {
      transformer: TcpReceiveTransformer::new(address),
    }
  }

  /// Sets the connection timeout in seconds.
  pub fn with_timeout_secs(mut self, secs: u64) -> Self {
    self.transformer = self.transformer.with_timeout_secs(secs);
    self
  }

  /// Sets the buffer size.
  pub fn with_buffer_size(mut self, size: usize) -> Self {
    self.transformer = self.transformer.with_buffer_size(size);
    self
  }

  /// Sets whether to read as lines.
  pub fn with_read_as_lines(mut self, read_as_lines: bool) -> Self {
    self.transformer = self.transformer.with_read_as_lines(read_as_lines);
    self
  }

  /// Sets the delimiter for reading data.
  pub fn with_delimiter(mut self, delimiter: Option<u8>) -> Self {
    self.transformer = self.transformer.with_delimiter(delimiter);
    self
  }

  /// Sets the maximum number of items to receive.
  pub fn with_max_items(mut self, max: Option<usize>) -> Self {
    self.transformer = self.transformer.with_max_items(max);
    self
  }

  /// Sets the error handling strategy for this node.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }
}

impl Clone for TcpReceive {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for TcpReceive {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for TcpReceive {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for TcpReceive {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<String>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<String> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<String> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}
