//! # TCP Request Transformer
//!
//! Transformer that makes TCP requests for each input item, sending data to a TCP
//! server and optionally receiving responses. Useful for request-response patterns
//! in stream processing pipelines.
//!
//! ## Overview
//!
//! The TCP Request Transformer provides:
//!
//! - **TCP Communication**: Connects to a remote TCP server for each item
//! - **Request Modes**: Send-only (fire-and-forget) or send-and-receive
//! - **Timeout Handling**: Configurable connection and response timeouts
//! - **Error Handling**: Configurable error strategies for connection failures
//!
//! ## Input/Output
//!
//! - **Input**: `Message<String>` - Data to send to the TCP server
//! - **Output**: `Message<String>` - Response data (in SendReceive mode) or echo of input (in SendOnly mode)
//!
//! ## Request Modes
//!
//! - **SendOnly**: Sends data and immediately outputs the input (fire-and-forget)
//! - **SendReceive**: Sends data, waits for response, and outputs the response
//!
//! ## Example
//!
//! ```rust
//! use crate::transformers::{TcpRequestTransformer, TcpRequestMode};
//!
//! // Send and receive
//! let transformer = TcpRequestTransformer::new(
//!   "127.0.0.1:8080".to_string(),
//!   TcpRequestMode::SendReceive,
//! );
//! // Input: ["hello", "world"]
//! // Sends each, receives responses, outputs: ["response1", "response2"]
//! ```
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt, future::Either};
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;
/// TCP request mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpRequestMode {
  /// Send only (fire and forget)
  SendOnly,
  /// Send and wait for response
  SendReceive,
}
/// A transformer that makes TCP requests from stream items.
///
/// For each input item, this transformer:
/// - Connects to a TCP server
/// - Sends the item as data
/// - Optionally waits for and returns a response
///
/// # Example
///
/// ```rust
/// use crate::transformers::{TcpRequestTransformer, TcpRequestMode};
///
/// // Send and receive
/// let transformer = TcpRequestTransformer::new(
///   "127.0.0.1:8080".to_string(),
///   TcpRequestMode::SendReceive,
/// );
/// // Input: ["hello", "world"]
/// // Sends each, receives responses, outputs: ["response1", "response2"]
/// ```
pub struct TcpRequestTransformer {
  /// Remote address to connect to.
  address: String,
  /// Connection timeout in seconds.
  timeout_secs: u64,
  /// Request mode (SendOnly or SendReceive).
  mode: TcpRequestMode,
  /// Response timeout in seconds (for SendReceive mode).
  response_timeout_secs: u64,
  /// Whether to append newline after each message.
  append_newline: bool,
  /// Delimiter for appending after each message (if not using newline).
  delimiter: Option<Vec<u8>>,
  /// Whether to read response as lines (for SendReceive mode).
  read_response_as_lines: bool,
  /// Response delimiter (for SendReceive mode, when not reading as lines).
  response_delimiter: Option<u8>,
  /// Configuration for the transformer.
  config: TransformerConfig<String>,
}
impl TcpRequestTransformer {
  /// Creates a new `TcpRequestTransformer`.
  ///
  /// # Arguments
  ///
  /// * `address` - The remote address to connect to (e.g., "127.0.0.1:8080").
  /// * `mode` - The request mode (SendOnly or SendReceive).
  pub fn new(address: impl Into<String>, mode: TcpRequestMode) -> Self {
    Self {
      address: address.into(),
      timeout_secs: 30,
      mode,
      response_timeout_secs: 10,
      append_newline: true,
      delimiter: None,
      read_response_as_lines: true,
      response_delimiter: None,
      config: TransformerConfig::default(),
    }
  }
  /// Sets the connection timeout in seconds.
  pub fn with_timeout_secs(mut self, secs: u64) -> Self {
    self.timeout_secs = secs;
    self
  }
  /// Sets the response timeout in seconds (for SendReceive mode).
  pub fn with_response_timeout_secs(mut self, secs: u64) -> Self {
    self.response_timeout_secs = secs;
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
  /// Sets whether to read response as lines (for SendReceive mode).
  pub fn with_read_response_as_lines(mut self, read_as_lines: bool) -> Self {
    self.read_response_as_lines = read_as_lines;
    self
  }
  /// Sets the response delimiter (for SendReceive mode).
  pub fn with_response_delimiter(mut self, delimiter: Option<u8>) -> Self {
    self.response_delimiter = delimiter;
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
impl Clone for TcpRequestTransformer {
  fn clone(&self) -> Self {
    Self {
      address: self.address.clone(),
      timeout_secs: self.timeout_secs,
      mode: self.mode,
      response_timeout_secs: self.response_timeout_secs,
      append_newline: self.append_newline,
      delimiter: self.delimiter.clone(),
      read_response_as_lines: self.read_response_as_lines,
      response_delimiter: self.response_delimiter,
      config: self.config.clone(),
    }
  }
}
impl Input for TcpRequestTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}
impl Output for TcpRequestTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}
#[async_trait]
impl Transformer for TcpRequestTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);
  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let address = self.address.clone();
    let timeout_secs = self.timeout_secs;
    let mode = self.mode;
    let response_timeout_secs = self.response_timeout_secs;
    let append_newline = self.append_newline;
    let delimiter = self.delimiter.clone();
    let read_response_as_lines = self.read_response_as_lines;
    let response_delimiter = self.response_delimiter;
    Box::pin(input.then(move |item| {
      let address = address.clone();
      let timeout_duration = Duration::from_secs(timeout_secs);
      let response_timeout = Duration::from_secs(response_timeout_secs);
      let delimiter = delimiter.clone();
      async move {
        // Parse address
        let addr: SocketAddr = match address.parse() {
          Ok(a) => a,
          Err(_) => return item, // Return original item on error
        };
        // Connect and send
        match timeout(timeout_duration, TcpStream::connect(addr)).await {
          Ok(Ok(mut stream)) => {
            let mut data = item.clone().into_bytes();
            if append_newline {
              data.push(b'\n');
            } else if let Some(ref delim) = delimiter {
              data.extend_from_slice(delim);
            }
            // Send data
            if stream.write_all(&data).await.is_err() {
              return item;
            }
            if stream.flush().await.is_err() {
              return item;
            }
            // Handle response based on mode
            match mode {
              TcpRequestMode::SendOnly => {
                // Just return the original item
                item
              }
              TcpRequestMode::SendReceive => {
                // Read response
                let (read_half, _write_half) = stream.into_split();
                let mut reader = BufReader::new(read_half);
                if read_response_as_lines {
                  let mut response = String::new();
                  match timeout(response_timeout, reader.read_line(&mut response)).await {
                    Ok(Ok(_)) => response
                      .trim_end_matches('\n')
                      .trim_end_matches('\r')
                      .to_string(),
                    _ => item, // Return original on timeout/error
                  }
                } else {
                  let mut buffer = vec![0u8; 8192];
                  let read_future = if let Some(delim) = response_delimiter {
                    Either::Left(reader.read_until(delim, &mut buffer))
                  } else {
                    Either::Right(reader.read(&mut buffer))
                  };
                  match timeout(response_timeout, read_future).await {
                    Ok(Ok(n)) if n > 0 => {
                      let data = if let Some(_delim) = response_delimiter {
                        buffer[..n - 1].to_vec()
                      } else {
                        buffer[..n].to_vec()
                      };
                      String::from_utf8(data).unwrap_or(item)
                    }
                    _ => item, // Return original on timeout/error
                  }
                }
              }
            }
          }
          _ => item, // Return original on connection error/timeout
        }
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
        .unwrap_or_else(|| "tcp_request_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
