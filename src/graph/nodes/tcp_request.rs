//! TCP request node for making TCP requests from stream items.
//!
//! This module provides [`TcpRequest`], a graph node that makes TCP requests from
//! stream items. It wraps [`TcpRequestTransformer`] for use in StreamWeave graphs.
//! It connects to a remote TCP address and sends/receives data, supporting both
//! send-only and send-receive modes.
//!
//! # Overview
//!
//! [`TcpRequest`] is useful for making TCP requests in graph-based pipelines.
//! It connects to a remote TCP address and sends data from stream items,
//! optionally receiving responses. It supports configurable connection options,
//! timeouts, delimiters, and reading modes.
//!
//! # Key Concepts
//!
//! - **TCP Requests**: Makes TCP requests to remote addresses
//! - **Send-Only Mode**: Sends data without waiting for responses
//! - **Send-Receive Mode**: Sends data and receives responses
//! - **Transformer Wrapper**: Wraps `TcpRequestTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`TcpRequest`]**: Node that makes TCP requests
//! - **[`TcpRequestMode`]**: Enum representing send-only or send-receive modes
//!
//! # Quick Start
//!
//! ## Basic Usage (Send-Only)
//!
//! ```rust
//! use streamweave::graph::nodes::TcpRequest;
//! use streamweave::transformers::TcpRequestMode;
//!
//! // Create a TCP request node in send-only mode
//! let tcp_request = TcpRequest::new("127.0.0.1:8080", TcpRequestMode::SendOnly);
//! ```
//!
//! ## Send-Receive Mode
//!
//! ```rust
//! use streamweave::graph::nodes::TcpRequest;
//! use streamweave::transformers::TcpRequestMode;
//!
//! // Create a TCP request node in send-receive mode
//! let tcp_request = TcpRequest::new("127.0.0.1:8080", TcpRequestMode::SendReceive)
//!     .with_response_timeout_secs(5)
//!     .with_read_response_as_lines(true);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::TcpRequest;
//! use streamweave::transformers::TcpRequestMode;
//! use streamweave::ErrorStrategy;
//!
//! // Create a TCP request node with error handling
//! let tcp_request = TcpRequest::new("127.0.0.1:8080", TcpRequestMode::SendReceive)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("tcp-request".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **TCP Connection**: Uses Tokio's async TCP for non-blocking network I/O
//! - **Mode Support**: Supports both send-only and send-receive modes for flexibility
//! - **Configurable Options**: Supports timeout, delimiter, newline appending
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`TcpRequest`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{TcpRequestMode, TcpRequestTransformer};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that makes TCP requests from stream items.
///
/// This node wraps `TcpRequestTransformer` for use in graphs.
pub struct TcpRequest {
  /// The underlying request transformer
  transformer: TcpRequestTransformer,
}

impl TcpRequest {
  /// Creates a new `TcpRequest` node.
  ///
  /// # Arguments
  ///
  /// * `address` - The remote address to connect to (e.g., "127.0.0.1:8080").
  /// * `mode` - The request mode (SendOnly or SendReceive).
  pub fn new(address: impl Into<String>, mode: TcpRequestMode) -> Self {
    Self {
      transformer: TcpRequestTransformer::new(address, mode),
    }
  }

  /// Sets the connection timeout in seconds.
  pub fn with_timeout_secs(mut self, secs: u64) -> Self {
    self.transformer = self.transformer.with_timeout_secs(secs);
    self
  }

  /// Sets the response timeout in seconds (for SendReceive mode).
  pub fn with_response_timeout_secs(mut self, secs: u64) -> Self {
    self.transformer = self.transformer.with_response_timeout_secs(secs);
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

  /// Sets whether to read response as lines (for SendReceive mode).
  pub fn with_read_response_as_lines(mut self, read_as_lines: bool) -> Self {
    self.transformer = self.transformer.with_read_response_as_lines(read_as_lines);
    self
  }

  /// Sets the response delimiter (for SendReceive mode).
  pub fn with_response_delimiter(mut self, delimiter: Option<u8>) -> Self {
    self.transformer = self.transformer.with_response_delimiter(delimiter);
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

impl Clone for TcpRequest {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for TcpRequest {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for TcpRequest {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for TcpRequest {
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
