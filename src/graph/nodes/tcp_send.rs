//! TCP send node for sending data over TCP connections in graphs.
//!
//! This module provides [`TcpSend`], a graph node that sends data from stream
//! items over TCP connections. It wraps [`TcpSendTransformer`] for use in
//! StreamWeave graphs. It connects to a remote TCP address and sends data,
//! making it ideal for network-based data output in graph-based pipelines.
//!
//! # Overview
//!
//! [`TcpSend`] is useful for sending data over TCP connections in graph-based
//! pipelines. It connects to a remote TCP address and sends data from stream
//! items, supporting configurable connection options, delimiters, and newline
//! appending.
//!
//! # Key Concepts
//!
//! - **TCP Transmission**: Sends data over TCP connections
//! - **Configurable Options**: Supports timeout, delimiter, newline appending
//! - **String Input**: Sends string data from stream items
//! - **Transformer Wrapper**: Wraps `TcpSendTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`TcpSend`]**: Node that sends data over TCP connections
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::TcpSend;
//!
//! // Create a TCP send node
//! let tcp_send = TcpSend::new("127.0.0.1:8080");
//! ```
//!
//! ## With Configuration
//!
//! ```rust
//! use streamweave::graph::nodes::TcpSend;
//!
//! // Create a TCP send node with configuration
//! let tcp_send = TcpSend::new("127.0.0.1:8080")
//!     .with_timeout_secs(10)
//!     .with_append_newline(true)
//!     .with_delimiter(Some(vec![0x0A, 0x0D])); // CRLF
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::TcpSend;
//! use streamweave::ErrorStrategy;
//!
//! // Create a TCP send node with error handling
//! let tcp_send = TcpSend::new("127.0.0.1:8080")
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("tcp-sender".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **TCP Connection**: Uses Tokio's async TCP for non-blocking network I/O
//! - **Configurable Options**: Supports timeout, delimiter, newline appending
//!   for flexibility
//! - **String-Based**: Works with string data for text-based protocols
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`TcpSend`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::TcpSendTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that sends data over TCP.
///
/// This node wraps `TcpSendTransformer` for use in graphs.
pub struct TcpSend {
  /// The underlying send transformer
  transformer: TcpSendTransformer,
}

impl TcpSend {
  /// Creates a new `TcpSend` node.
  ///
  /// # Arguments
  ///
  /// * `address` - The remote address to connect to (e.g., "127.0.0.1:8080").
  pub fn new(address: impl Into<String>) -> Self {
    Self {
      transformer: TcpSendTransformer::new(address),
    }
  }

  /// Sets the connection timeout in seconds.
  pub fn with_timeout_secs(mut self, secs: u64) -> Self {
    self.transformer = self.transformer.with_timeout_secs(secs);
    self
  }

  /// Sets whether to append newline after each message.
  pub fn with_append_newline(mut self, append: bool) -> Self {
    self.transformer = self.transformer.with_append_newline(append);
    self
  }

  /// Sets the delimiter to append after each message.
  pub fn with_delimiter(mut self, delimiter: Option<Vec<u8>>) -> Self {
    self.transformer = self.transformer.with_delimiter(delimiter);
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

impl Clone for TcpSend {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for TcpSend {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for TcpSend {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for TcpSend {
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
