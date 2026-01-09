//! TCP receive transformer for StreamWeave
//!
//! Receives data from TCP connections, useful for graph composition.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;

/// A transformer that receives data from TCP connections.
///
/// This transformer connects to a TCP server and reads data,
/// producing it as a stream. Useful for graph composition when
/// you need to receive TCP data in the middle of a graph.
///
/// Note: For simple TCP receiving, TCPProducer is more efficient.
/// Use this transformer when you need to compose TCP receiving
/// with other transformations in a graph.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::TcpReceiveTransformer;
///
/// let transformer = TcpReceiveTransformer::new("127.0.0.1:8080".to_string());
/// // Connects and reads data, outputs: ["line1", "line2", ...]
/// ```
pub struct TcpReceiveTransformer {
  /// Remote address to connect to.
  address: String,
  /// Connection timeout in seconds.
  timeout_secs: u64,
  /// Buffer size for reading data.
  buffer_size: usize,
  /// Whether to read as lines (line-delimited) or raw bytes.
  read_as_lines: bool,
  /// Delimiter for reading data (used when read_as_lines is false).
  delimiter: Option<u8>,
  /// Maximum number of items to receive (None for unlimited).
  max_items: Option<usize>,
  /// Configuration for the transformer.
  config: TransformerConfig<String>,
}

impl TcpReceiveTransformer {
  /// Creates a new `TcpReceiveTransformer`.
  ///
  /// # Arguments
  ///
  /// * `address` - The remote address to connect to (e.g., "127.0.0.1:8080").
  pub fn new(address: impl Into<String>) -> Self {
    Self {
      address: address.into(),
      timeout_secs: 30,
      buffer_size: 8192,
      read_as_lines: true,
      delimiter: None,
      max_items: None,
      config: TransformerConfig::default(),
    }
  }

  /// Sets the connection timeout in seconds.
  pub fn with_timeout_secs(mut self, secs: u64) -> Self {
    self.timeout_secs = secs;
    self
  }

  /// Sets the buffer size.
  pub fn with_buffer_size(mut self, size: usize) -> Self {
    self.buffer_size = size;
    self
  }

  /// Sets whether to read as lines.
  pub fn with_read_as_lines(mut self, read_as_lines: bool) -> Self {
    self.read_as_lines = read_as_lines;
    self
  }

  /// Sets the delimiter for reading data.
  pub fn with_delimiter(mut self, delimiter: Option<u8>) -> Self {
    self.delimiter = delimiter;
    self
  }

  /// Sets the maximum number of items to receive.
  pub fn with_max_items(mut self, max: Option<usize>) -> Self {
    self.max_items = max;
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

impl Clone for TcpReceiveTransformer {
  fn clone(&self) -> Self {
    Self {
      address: self.address.clone(),
      timeout_secs: self.timeout_secs,
      buffer_size: self.buffer_size,
      read_as_lines: self.read_as_lines,
      delimiter: self.delimiter,
      max_items: self.max_items,
      config: self.config.clone(),
    }
  }
}

impl Input for TcpReceiveTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for TcpReceiveTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for TcpReceiveTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, _input: Self::InputStream) -> Self::OutputStream {
    let address = self.address.clone();
    let timeout_secs = self.timeout_secs;
    let buffer_size = self.buffer_size;
    let read_as_lines = self.read_as_lines;
    let delimiter = self.delimiter;
    let max_items = self.max_items;

    // This transformer receives data from TCP, so it ignores input
    // and produces a stream from the TCP connection
    Box::pin(async_stream::stream! {
      // Parse address
      let addr: SocketAddr = match address.parse() {
        Ok(a) => a,
        Err(_) => return,
      };

      let timeout_duration = Duration::from_secs(timeout_secs);

      // Connect to TCP server
      let stream = match timeout(timeout_duration, TcpStream::connect(addr)).await {
        Ok(Ok(s)) => s,
        _ => return,
      };

      let (read_half, _write_half) = stream.into_split();
      let mut reader = BufReader::new(read_half);
      let mut count = 0;

      if read_as_lines {
        // Read line by line
        let mut line = String::new();
        loop {
          if let Some(max) = max_items
            && count >= max
          {
            break;
          }

          match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
              let trimmed = line.trim_end_matches('\n').trim_end_matches('\r').to_string();
              if !trimmed.is_empty() {
                yield trimmed;
                count += 1;
              }
              line.clear();
            }
            Err(_) => break,
          }
        }
      } else {
        // Read raw bytes with optional delimiter
        let mut buffer = vec![0u8; buffer_size];
        loop {
          if let Some(max) = max_items
            && count >= max
          {
            break;
          }

          match if let Some(delim) = delimiter {
            reader.read_until(delim, &mut buffer).await
          } else {
            reader.read(&mut buffer).await
          } {
            Ok(0) => break, // EOF
            Ok(n) => {
              let data = if let Some(_delim) = delimiter {
                buffer[..n - 1].to_vec()
              } else {
                buffer[..n].to_vec()
              };
              if let Ok(text) = String::from_utf8(data)
                && !text.is_empty()
              {
                yield text;
                count += 1;
              }
              buffer.clear();
            }
            Err(_) => break,
          }
        }
      }
    })
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
        .unwrap_or_else(|| "tcp_receive_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
