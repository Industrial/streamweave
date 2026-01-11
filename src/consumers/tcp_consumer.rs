//! TCP consumer for streaming data over TCP connections.
//!
//! This module provides [`TcpConsumer`] and [`TcpConsumerConfig`], a consumer that
//! sends stream data to a TCP server. Items are written over a TCP connection with
//! configurable delimiters and connection timeouts.
//!
//! # Overview
//!
//! [`TcpConsumer`] is useful for streaming data to TCP servers, network services, and
//! remote systems. It establishes a single TCP connection and sends all stream items
//! over that connection, with configurable message formatting and connection settings.
//!
//! # Key Concepts
//!
//! - **TCP Connection**: Establishes a single TCP connection to the remote server
//! - **Configurable Formatting**: Supports newline or custom delimiters between messages
//! - **Connection Timeout**: Configurable timeout for connection establishment
//! - **String Input**: Items must be strings for network transmission
//!
//! # Core Types
//!
//! - **[`TcpConsumer`]**: Consumer that sends stream items to a TCP server
//! - **[`TcpConsumerConfig`]**: Configuration for TCP consumer behavior
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::consumers::{TcpConsumer, TcpConsumerConfig};
//! use futures::stream;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create TCP configuration
//! let tcp_config = TcpConsumerConfig::default()
//!     .with_address("127.0.0.1:8080")
//!     .with_timeout_secs(10)
//!     .with_append_newline(true);
//!
//! // Create a consumer
//! let mut consumer = TcpConsumer::new(tcp_config);
//!
//! // Create a stream of messages
//! let stream = stream::iter(vec![
//!     "message1".to_string(),
//!     "message2".to_string(),
//!     "message3".to_string(),
//! ]);
//!
//! // Consume the stream (messages sent over TCP)
//! consumer.consume(Box::pin(stream)).await;
//! # Ok(())
//! # }
//! ```
//!
//! ## With Custom Delimiter
//!
//! ```rust
//! use streamweave::consumers::{TcpConsumer, TcpConsumerConfig};
//!
//! // Create TCP configuration with custom delimiter
//! let tcp_config = TcpConsumerConfig::default()
//!     .with_address("127.0.0.1:8080")
//!     .with_append_newline(false)
//!     .with_delimiter(Some(vec![0x00]));  // Null byte delimiter
//!
//! let consumer = TcpConsumer::new(tcp_config);
//! ```
//!
//! # Design Decisions
//!
//! - **Single Connection**: Uses one TCP connection for all items, efficient for streaming
//! - **Configurable Formatting**: Supports both newline and custom delimiters for
//!   different network protocols
//! - **Connection Timeout**: Prevents indefinite blocking on connection failures
//! - **String Input**: Requires string input for network transmission
//!
//! # Integration with StreamWeave
//!
//! [`TcpConsumer`] implements the [`Consumer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies and
//! configuration options provided by [`ConsumerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Consumer, ConsumerConfig, Input};
use async_trait::async_trait;
use futures::StreamExt;
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Configuration for TCP consumer behavior.
#[derive(Debug, Clone)]
pub struct TcpConsumerConfig {
  /// Remote address to connect to (e.g., "127.0.0.1:8080").
  pub address: String,
  /// Connection timeout in seconds.
  pub timeout_secs: u64,
  /// Whether to append newline after each message.
  pub append_newline: bool,
  /// Delimiter to append after each message (if not using newline).
  pub delimiter: Option<Vec<u8>>,
}

impl Default for TcpConsumerConfig {
  fn default() -> Self {
    Self {
      address: "127.0.0.1:8080".to_string(),
      timeout_secs: 30,
      append_newline: true,
      delimiter: None,
    }
  }
}

impl TcpConsumerConfig {
  /// Sets the remote address.
  #[must_use]
  pub fn with_address(mut self, address: impl Into<String>) -> Self {
    self.address = address.into();
    self
  }

  /// Sets the connection timeout in seconds.
  #[must_use]
  pub fn with_timeout_secs(mut self, secs: u64) -> Self {
    self.timeout_secs = secs;
    self
  }

  /// Sets whether to append newline after each message.
  #[must_use]
  pub fn with_append_newline(mut self, append: bool) -> Self {
    self.append_newline = append;
    self
  }

  /// Sets the delimiter to append after each message.
  #[must_use]
  pub fn with_delimiter(mut self, delimiter: Option<Vec<u8>>) -> Self {
    self.delimiter = delimiter;
    self
  }
}

/// A consumer that sends data to a TCP connection.
///
/// This consumer connects to a TCP server and sends each item from the stream
/// as data over the connection.
pub struct TcpConsumer {
  /// TCP consumer configuration.
  pub tcp_config: TcpConsumerConfig,
  /// Consumer configuration.
  pub config: ConsumerConfig<String>,
}

impl TcpConsumer {
  /// Creates a new `TcpConsumer` with the given configuration.
  ///
  /// # Arguments
  ///
  /// * `tcp_config` - The TCP configuration.
  pub fn new(tcp_config: TcpConsumerConfig) -> Self {
    Self {
      tcp_config,
      config: ConsumerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this consumer.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this consumer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }
}

impl Input for TcpConsumer {
  type Input = String;
  type InputStream = Pin<Box<dyn futures::Stream<Item = String> + Send>>;
}

#[async_trait]
impl Consumer for TcpConsumer {
  type InputPorts = (String,);

  async fn consume(&mut self, input: Self::InputStream) {
    let tcp_config = self.tcp_config.clone();
    let component_name = if self.config.name.is_empty() {
      "tcp_consumer".to_string()
    } else {
      self.config.name.clone()
    };
    let timeout_duration = Duration::from_secs(tcp_config.timeout_secs);

    // Parse address
    let addr: SocketAddr = match tcp_config.address.parse() {
      Ok(a) => a,
      Err(e) => {
        tracing::error!(
          component = %component_name,
          address = %tcp_config.address,
          error = %e,
          "Failed to parse TCP address"
        );
        return;
      }
    };

    // Connect to TCP server
    let mut stream = match timeout(timeout_duration, TcpStream::connect(addr)).await {
      Ok(Ok(s)) => s,
      Ok(Err(e)) => {
        tracing::error!(
          component = %component_name,
          address = %tcp_config.address,
          error = %e,
          "Failed to connect to TCP server"
        );
        return;
      }
      Err(_) => {
        tracing::error!(
          component = %component_name,
          address = %tcp_config.address,
          "Connection timeout"
        );
        return;
      }
    };

    let mut input = std::pin::pin!(input);

    // Send each item
    while let Some(item) = input.next().await {
      let mut data = item.into_bytes();

      if tcp_config.append_newline {
        data.push(b'\n');
      } else if let Some(ref delim) = tcp_config.delimiter {
        data.extend_from_slice(delim);
      }

      if let Err(e) = stream.write_all(&data).await {
        tracing::error!(
          component = %component_name,
          error = %e,
          "Error writing to TCP connection"
        );
        break;
      }

      if let Err(e) = stream.flush().await {
        tracing::error!(
          component = %component_name,
          error = %e,
          "Error flushing TCP connection"
        );
        break;
      }
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<String> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: if self.config.name.is_empty() {
        "tcp_consumer".to_string()
      } else {
        self.config.name.clone()
      },
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
