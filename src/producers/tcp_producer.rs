//! TCP producer for StreamWeave
//!
//! Receives data from TCP connections.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Output, Producer, ProducerConfig};
use async_stream::stream;
use futures::Stream;
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Configuration for TCP producer behavior.
#[derive(Debug, Clone)]
pub struct TcpProducerConfig {
  /// Remote address to connect to (e.g., "127.0.0.1:8080").
  pub address: String,
  /// Connection timeout in seconds.
  pub timeout_secs: u64,
  /// Buffer size for reading data.
  pub buffer_size: usize,
  /// Whether to read as lines (line-delimited) or raw bytes.
  pub read_as_lines: bool,
  /// Delimiter for reading data (used when read_as_lines is false).
  pub delimiter: Option<u8>,
}

impl Default for TcpProducerConfig {
  fn default() -> Self {
    Self {
      address: "127.0.0.1:8080".to_string(),
      timeout_secs: 30,
      buffer_size: 8192,
      read_as_lines: true,
      delimiter: None,
    }
  }
}

impl TcpProducerConfig {
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

/// A producer that receives data from a TCP connection.
///
/// This producer connects to a TCP server and reads data from the connection,
/// producing it as a stream of strings (lines) or bytes.
pub struct TcpProducer {
  /// TCP producer configuration.
  pub tcp_config: TcpProducerConfig,
  /// Producer configuration.
  pub config: ProducerConfig<String>,
}

impl TcpProducer {
  /// Creates a new `TcpProducer` with the given configuration.
  ///
  /// # Arguments
  ///
  /// * `tcp_config` - The TCP configuration.
  pub fn new(tcp_config: TcpProducerConfig) -> Self {
    Self {
      tcp_config,
      config: ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this producer.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Output for TcpProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait::async_trait]
impl Producer for TcpProducer {
  type OutputPorts = (String,);

  fn produce(&mut self) -> Self::OutputStream {
    let tcp_config = self.tcp_config.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "tcp_producer".to_string());
    let timeout_duration = Duration::from_secs(tcp_config.timeout_secs);

    Box::pin(stream! {
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
      let stream = match timeout(timeout_duration, TcpStream::connect(addr)).await {
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

      let (read_half, _write_half) = stream.into_split();
      let mut reader = BufReader::new(read_half);

      if tcp_config.read_as_lines {
        // Read line by line
        let mut line = String::new();
        loop {
          match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
              let trimmed = line.trim_end_matches('\n').trim_end_matches('\r').to_string();
              if !trimmed.is_empty() {
                yield trimmed;
              }
              line.clear();
            }
            Err(e) => {
              tracing::error!(
                component = %component_name,
                error = %e,
                "Error reading from TCP connection"
              );
              break;
            }
          }
        }
      } else {
        // Read raw bytes with optional delimiter
        let mut buffer = vec![0u8; tcp_config.buffer_size];
        loop {
          match if let Some(delim) = tcp_config.delimiter {
            reader.read_until(delim, &mut buffer).await
          } else {
            reader.read(&mut buffer).await
          } {
            Ok(0) => break, // EOF
            Ok(n) => {
              let data = if let Some(_delim) = tcp_config.delimiter {
                // Remove delimiter
                buffer[..n - 1].to_vec()
              } else {
                buffer[..n].to_vec()
              };
              if let Ok(text) = String::from_utf8(data)
                && !text.is_empty()
              {
                yield text;
              }
              buffer.clear();
            }
            Err(e) => {
              tracing::error!(
                component = %component_name,
                error = %e,
                "Error reading from TCP connection"
              );
              break;
            }
          }
        }
      }
    })
  }

  fn set_config_impl(&mut self, config: ProducerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<String> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    match self.config.error_strategy() {
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
        .unwrap_or_else(|| "tcp_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
