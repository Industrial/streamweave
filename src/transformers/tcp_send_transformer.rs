//! # TCP Send Transformer
//!
//! Transformer that sends data from input items over TCP connections in a
//! fire-and-forget manner, passing the same data through to the output stream.
//! This is a simple send-only operation without waiting for responses.
//!
//! ## Overview
//!
//! The TCP Send Transformer provides:
//!
//! - **TCP Sending**: Sends data over TCP connections for each input item
//! - **Pass-Through**: Outputs the same items that were sent
//! - **Fire-and-Forget**: Does not wait for responses (for request/response, use `TcpRequestTransformer`)
//! - **Delimiter Support**: Optional newline or custom delimiter appending
//! - **Error Handling**: Configurable error strategies for send failures
//!
//! ## Input/Output
//!
//! - **Input**: `Message<String>` - Data to send over TCP
//! - **Output**: `Message<String>` - The same data (pass-through)
//!
//! ## Use Cases
//!
//! - **Logging**: Send log messages to a TCP log server
//! - **Event Streaming**: Stream events to a TCP endpoint
//! - **Simple Communication**: Fire-and-forget TCP messaging
//!
//! ## Example
//!
//! ```rust
//! use streamweave::transformers::TcpSendTransformer;
//!
//! let transformer = TcpSendTransformer::new("127.0.0.1:8080".to_string());
//! // Input: ["hello", "world"]
//! // Sends each string over TCP, outputs: ["hello", "world"]
//! ```

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::timeout;

/// A transformer that sends data from stream items over TCP (fire-and-forget).
///
/// For each input item, this transformer sends it over a TCP connection
/// and passes through the original item. This is a simple send-only operation.
///
/// For request/response patterns, use `TcpRequestTransformer` instead.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::TcpSendTransformer;
///
/// let transformer = TcpSendTransformer::new("127.0.0.1:8080".to_string());
/// // Input: ["hello", "world"]
/// // Sends each string over TCP, outputs: ["hello", "world"]
/// ```
pub struct TcpSendTransformer {
  /// Remote address to connect to.
  address: String,
  /// Connection timeout in seconds.
  timeout_secs: u64,
  /// Whether to append newline after each message.
  append_newline: bool,
  /// Delimiter to append after each message (if not using newline).
  delimiter: Option<Vec<u8>>,
  /// Configuration for the transformer.
  config: TransformerConfig<String>,
}

impl TcpSendTransformer {
  /// Creates a new `TcpSendTransformer`.
  ///
  /// # Arguments
  ///
  /// * `address` - The remote address to connect to (e.g., "127.0.0.1:8080").
  pub fn new(address: impl Into<String>) -> Self {
    Self {
      address: address.into(),
      timeout_secs: 30,
      append_newline: true,
      delimiter: None,
      config: TransformerConfig::default(),
    }
  }

  /// Sets the connection timeout in seconds.
  pub fn with_timeout_secs(mut self, secs: u64) -> Self {
    self.timeout_secs = secs;
    self
  }

  /// Sets whether to append newline after each message.
  pub fn with_append_newline(mut self, append: bool) -> Self {
    self.append_newline = append;
    self
  }

  /// Sets the delimiter to append after each message.
  pub fn with_delimiter(mut self, delimiter: Option<Vec<u8>>) -> Self {
    self.delimiter = delimiter;
    self
  }

  /// Sets the error handling strategy for this transformer.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Clone for TcpSendTransformer {
  fn clone(&self) -> Self {
    Self {
      address: self.address.clone(),
      timeout_secs: self.timeout_secs,
      append_newline: self.append_newline,
      delimiter: self.delimiter.clone(),
      config: self.config.clone(),
    }
  }
}

impl Input for TcpSendTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for TcpSendTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for TcpSendTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let address = self.address.clone();
    let timeout_secs = self.timeout_secs;
    let append_newline = self.append_newline;
    let delimiter = self.delimiter.clone();
    Box::pin(input.then(move |item| {
      let address = address.clone();
      let delimiter = delimiter.clone();
      let timeout_duration = Duration::from_secs(timeout_secs);
      async move {
        // Parse address
        let addr: SocketAddr = match address.parse() {
          Ok(a) => a,
          Err(_) => return item, // Return original item on error
        };

        // Connect and send
        if let Ok(Ok(mut stream)) = timeout(timeout_duration, TcpStream::connect(addr)).await {
          let mut data = item.clone().into_bytes();

          if append_newline {
            data.push(b'\n');
          } else if let Some(ref delim) = delimiter {
            data.extend_from_slice(delim);
          }

          let _ = stream.write_all(&data).await;
          let _ = stream.flush().await;
        }

        item // Pass through original item
      }
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<String> {
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
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "tcp_send_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
