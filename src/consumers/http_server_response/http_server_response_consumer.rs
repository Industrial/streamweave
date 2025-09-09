use crate::{
  consumer::{Consumer, ConsumerConfig},
  http::connection::ConnectionManager,
  input::Input,
};
use async_trait::async_trait;
use futures::StreamExt;
use std::{pin::Pin, sync::Arc};
use tokio::io::AsyncWriteExt;
use tokio_stream::Stream;
use tracing::{debug, error, warn};

use super::http_server_response::HttpServerResponse;

/// Consumer that sends HTTP responses back to TCP connections
#[derive(Debug)]
pub struct HttpServerResponseConsumer {
  connection_manager: Arc<ConnectionManager>,
  config: ConsumerConfig<HttpServerResponse>,
}

impl HttpServerResponseConsumer {
  /// Create a new HTTP server response consumer
  pub fn new(connection_manager: Arc<ConnectionManager>) -> Self {
    Self {
      connection_manager,
      config: ConsumerConfig::default(),
    }
  }

  /// Set the consumer configuration
  pub fn with_config(mut self, config: ConsumerConfig<HttpServerResponse>) -> Self {
    self.config = config;
    self
  }

  /// Format an HTTP response into bytes
  fn format_http_response(&self, response: &HttpServerResponse) -> Vec<u8> {
    let mut response_bytes = Vec::new();

    // Status line
    let status_line = format!(
      "HTTP/1.1 {} {}\r\n",
      response.status.as_u16(),
      response.status.canonical_reason().unwrap_or("Unknown")
    );
    response_bytes.extend_from_slice(status_line.as_bytes());

    // Headers
    for (name, value) in response.headers.iter() {
      let header_line = format!("{}: {}\r\n", name, value.to_str().unwrap_or(""));
      response_bytes.extend_from_slice(header_line.as_bytes());
    }

    // Content-Length header if not already present
    if !response.headers.contains_key("content-length") {
      let content_length = format!("Content-Length: {}\r\n", response.body.len());
      response_bytes.extend_from_slice(content_length.as_bytes());
    }

    // End of headers
    response_bytes.extend_from_slice(b"\r\n");

    // Body
    response_bytes.extend_from_slice(&response.body);

    response_bytes
  }

  /// Send a response back to the TCP connection
  async fn send_response(
    &self,
    response: HttpServerResponse,
  ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("Sending HTTP response for request {}", response.request_id);

    // Get the connection ID for this request
    let connection_id = match self
      .connection_manager
      .get_connection_for_request(response.request_id)
      .await
    {
      Some(id) => id,
      None => {
        warn!("No connection found for request {}", response.request_id);
        return Ok(());
      }
    };

    // Get the TCP stream for this connection
    let stream = match self.connection_manager.get_connection(connection_id).await {
      Some(stream) => stream,
      None => {
        warn!("Connection {} not found", connection_id);
        return Ok(());
      }
    };

    // Format the HTTP response
    let response_bytes = self.format_http_response(&response);

    // Send the response
    {
      let mut stream = stream.lock().await;
      if let Err(e) = stream.write_all(&response_bytes).await {
        error!(
          "Failed to send response to connection {}: {}",
          connection_id, e
        );
        return Err(Box::new(e));
      }

      if let Err(e) = stream.flush().await {
        error!(
          "Failed to flush response to connection {}: {}",
          connection_id, e
        );
        return Err(Box::new(e));
      }
    }

    debug!(
      "Successfully sent HTTP response for request {}",
      response.request_id
    );

    // Clean up the request mapping
    self
      .connection_manager
      .remove_request(response.request_id)
      .await;

    Ok(())
  }
}

impl Input for HttpServerResponseConsumer {
  type Input = HttpServerResponse;
  type InputStream = Pin<Box<dyn Stream<Item = HttpServerResponse> + Send>>;
}

#[async_trait]
impl Consumer for HttpServerResponseConsumer {
  async fn consume(&mut self, mut stream: Self::InputStream) {
    while let Some(response) = stream.next().await {
      if let Err(e) = self.send_response(response).await {
        error!("Failed to send HTTP response: {}", e);
      }
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<Self::Input>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> ConsumerConfig<Self::Input> {
    self.config.clone()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::http::connection::ConnectionManager;
  use bytes::Bytes;
  use std::sync::Arc;
  use uuid::Uuid;

  #[tokio::test]
  async fn test_http_server_response_consumer_creation() {
    let connection_manager = Arc::new(ConnectionManager::new(100));
    let consumer = HttpServerResponseConsumer::new(connection_manager);

    assert!(consumer.connection_manager.connection_count().await == 0);
  }

  #[tokio::test]
  async fn test_format_http_response() {
    let connection_manager = Arc::new(ConnectionManager::new(100));
    let consumer = HttpServerResponseConsumer::new(connection_manager);

    let request_id = Uuid::new_v4();
    let response = HttpServerResponse::ok(request_id, Bytes::from("Hello, World!"))
      .with_header("Content-Type", "text/plain");

    let formatted = consumer.format_http_response(&response);
    let response_str = String::from_utf8(formatted).unwrap();

    assert!(response_str.contains("HTTP/1.1 200 OK"));
    assert!(response_str.contains("content-type: text/plain"));
    assert!(response_str.contains("Content-Length: 13"));
    assert!(response_str.contains("Hello, World!"));
  }

  #[tokio::test]
  async fn test_consumer_trait_implementation() {
    let connection_manager = Arc::new(ConnectionManager::new(100));
    let mut consumer = HttpServerResponseConsumer::new(connection_manager);

    // Test that it implements the Consumer trait
    let config = ConsumerConfig::default();
    consumer.set_config_impl(config);

    let _config = consumer.get_config_impl();

    // This test verifies the trait implementation compiles correctly
    assert!(true);
  }

  #[tokio::test]
  async fn test_input_trait_implementation() {
    fn assert_input_trait<T: Input<Input = HttpServerResponse>>(_: T) {}

    let connection_manager = Arc::new(ConnectionManager::new(100));
    let consumer = HttpServerResponseConsumer::new(connection_manager);

    // This will compile only if HttpServerResponseConsumer implements Input<Input = HttpServerResponse>
    assert_input_trait(consumer);
  }
}
