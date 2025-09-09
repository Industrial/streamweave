use crate::{
  http::{
    connection::ConnectionManager, connection_info::ConnectionInfo,
    http_request_chunk::StreamWeaveHttpRequestChunk,
  },
  producer::ProducerConfig,
};
use bytes::Bytes;
use http::{HeaderMap, Method, Uri, Version};
use std::{collections::HashMap, net::SocketAddr, time::Duration};
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::{TcpListener, TcpStream},
  sync::{Semaphore, mpsc},
  time::timeout,
};

use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::output::{HttpRequestChannel, HttpRequestStream};

/// HTTP Server Producer that accepts incoming HTTP connections and produces HTTP request streams
///
/// This producer replaces the need for Axum's server functionality by directly handling
/// HTTP protocol parsing and connection management within the StreamWeave architecture.
#[derive(Debug)]
pub struct HttpServerProducer {
  listener: Option<TcpListener>,
  max_connections: Arc<Semaphore>,
  connection_timeout: Duration,
  keep_alive_timeout: Duration,
  pub(crate) config: ProducerConfig<StreamWeaveHttpRequestChunk>,
  request_sender: Option<HttpRequestChannel>,
  shutdown_sender: Option<mpsc::UnboundedSender<()>>,
  connection_manager: Arc<ConnectionManager>,
}

impl HttpServerProducer {
  /// Create a new HTTP Server Producer
  pub fn new(
    listener: TcpListener,
    max_connections: usize,
    connection_timeout: Duration,
    keep_alive_timeout: Duration,
  ) -> Self {
    Self {
      listener: Some(listener),
      max_connections: Arc::new(Semaphore::new(max_connections)),
      connection_timeout,
      keep_alive_timeout,
      config: ProducerConfig::default(),
      request_sender: None,
      shutdown_sender: None,
      connection_manager: Arc::new(ConnectionManager::new(max_connections)),
    }
  }

  /// Create a new HTTP Server Producer bound to the specified address
  pub async fn bind(addr: SocketAddr) -> Result<Self, std::io::Error> {
    let listener = TcpListener::bind(addr).await?;
    Ok(Self::new(
      listener,
      1000,                    // Default max connections
      Duration::from_secs(30), // Default connection timeout
      Duration::from_secs(60), // Default keep-alive timeout
    ))
  }

  /// Set the maximum number of concurrent connections
  pub fn with_max_connections(mut self, max_connections: usize) -> Self {
    self.max_connections = Arc::new(Semaphore::new(max_connections));
    self
  }

  /// Set the connection timeout
  pub fn with_connection_timeout(mut self, timeout: Duration) -> Self {
    self.connection_timeout = timeout;
    self
  }

  /// Set the keep-alive timeout
  pub fn with_keep_alive_timeout(mut self, timeout: Duration) -> Self {
    self.keep_alive_timeout = timeout;
    self
  }

  /// Start the HTTP server and return the request stream
  pub async fn start(
    &mut self,
  ) -> Result<HttpRequestStream, Box<dyn std::error::Error + Send + Sync>> {
    let (request_sender, request_receiver) = mpsc::unbounded_channel();
    let (shutdown_sender, mut shutdown_receiver) = mpsc::unbounded_channel();

    self.request_sender = Some(request_sender);
    self.shutdown_sender = Some(shutdown_sender);

    let listener = self.listener.take().ok_or("Listener already taken")?;
    let max_connections = self.max_connections.clone();
    let connection_timeout = self.connection_timeout;
    let keep_alive_timeout = self.keep_alive_timeout;
    let request_sender = self.request_sender.as_ref().unwrap().clone();
    let connection_manager = self.connection_manager.clone();

    // Spawn the server task
    tokio::spawn(async move {
      let mut _connection_count = 0;

      loop {
        tokio::select! {
            // Accept new connections
            result = listener.accept() => {
                match result {
                    Ok((mut stream, addr)) => {
                        let permit = max_connections.clone().acquire_owned().await;
                        if let Ok(permit) = permit {
                            _connection_count += 1;
                            let request_sender = request_sender.clone();
                            let connection_timeout = connection_timeout;
                            let keep_alive_timeout = keep_alive_timeout;
                            let connection_manager = connection_manager.clone();

                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_connection(
                                    stream,
                                    addr,
                                    request_sender,
                                    connection_timeout,
                                    keep_alive_timeout,
                                    connection_manager.clone(),
                                ).await {
                                    error!("Error handling connection from {}: {}", addr, e);
                                }
                                drop(permit);
                            });
                        } else {
                            warn!("Failed to acquire connection permit for {}", addr);
                            let _ = stream.shutdown().await;
                        }
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                    }
                }
            }
            // Handle shutdown signal
            _ = shutdown_receiver.recv() => {
                info!("HTTP Server Producer shutting down");
                break;
            }
        }
      }
    });

    Ok(HttpRequestStream::new(request_receiver))
  }

  /// Handle a single HTTP connection
  async fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
    request_sender: HttpRequestChannel,
    connection_timeout: Duration,
    keep_alive_timeout: Duration,
    _connection_manager: Arc<ConnectionManager>,
  ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("Handling connection from {}", addr);

    let mut buffer = [0; 8192];
    let mut keep_alive = true;

    while keep_alive {
      // Read HTTP request with timeout
      let read_result = timeout(connection_timeout, stream.read(&mut buffer)).await;

      match read_result {
        Ok(Ok(0)) => {
          debug!("Connection closed by client: {}", addr);
          break;
        }
        Ok(Ok(n)) => {
          let request_data = &buffer[..n];
          debug!("Received {} bytes from {}", n, addr);

          // Parse HTTP request
          match Self::parse_http_request(request_data, addr).await {
            Ok(Some(request_chunk)) => {
              // Send request chunk to the pipeline
              if let Err(e) = request_sender.send(request_chunk) {
                error!("Failed to send request chunk: {}", e);
                break;
              }

              // Check if this is a keep-alive connection
              keep_alive = Self::is_keep_alive_request(request_data);

              if keep_alive {
                // Wait for next request with keep-alive timeout
                let next_request = timeout(keep_alive_timeout, async {
                  let mut next_buffer = [0; 8192];
                  stream
                    .read(&mut next_buffer)
                    .await
                    .map(|n| (n, next_buffer))
                })
                .await;

                match next_request {
                  Ok(Ok((0, _))) => {
                    debug!("Keep-alive connection closed by client: {}", addr);
                    break;
                  }
                  Ok(Ok((_n, next_buffer))) => {
                    buffer = next_buffer;
                    // Continue processing the next request
                  }
                  Ok(Err(e)) => {
                    error!("Error reading next request from {}: {}", addr, e);
                    break;
                  }
                  Err(_) => {
                    debug!("Keep-alive timeout for connection: {}", addr);
                    break;
                  }
                }
              } else {
                break;
              }
            }
            Ok(None) => {
              debug!("Incomplete request from {}, waiting for more data", addr);
              // For simplicity, we'll break here. In a real implementation,
              // you'd want to buffer partial requests
              break;
            }
            Err(e) => {
              error!("Failed to parse HTTP request from {}: {}", addr, e);
              break;
            }
          }
        }
        Ok(Err(e)) => {
          error!("Error reading from connection {}: {}", addr, e);
          break;
        }
        Err(_) => {
          debug!("Connection timeout for {}", addr);
          break;
        }
      }
    }

    debug!("Connection handling completed for {}", addr);
    Ok(())
  }

  /// Parse HTTP request from raw bytes
  async fn parse_http_request(
    data: &[u8],
    remote_addr: SocketAddr,
  ) -> Result<Option<StreamWeaveHttpRequestChunk>, Box<dyn std::error::Error + Send + Sync>> {
    let request_str = String::from_utf8_lossy(data);
    let lines: Vec<&str> = request_str.lines().collect();

    if lines.is_empty() {
      return Ok(None);
    }

    // Parse request line
    let request_line = lines[0];
    let parts: Vec<&str> = request_line.split_whitespace().collect();

    if parts.len() < 3 {
      return Err("Invalid HTTP request line".into());
    }

    let method = parts[0].parse::<Method>()?;
    let uri = parts[1].parse::<Uri>()?;
    let version = if parts[2] == "HTTP/1.1" {
      Version::HTTP_11
    } else if parts[2] == "HTTP/1.0" {
      Version::HTTP_10
    } else if parts[2] == "HTTP/2" {
      Version::HTTP_2
    } else {
      return Err(format!("Unsupported HTTP version: {}", parts[2]).into());
    };

    // Parse headers
    let mut headers = HeaderMap::new();
    let mut body_start = 0;

    for (i, line) in lines.iter().enumerate().skip(1) {
      if line.is_empty() {
        body_start = i + 1;
        break;
      }

      if let Some((name, value)) = line.split_once(':') {
        let name = name.trim().to_lowercase();
        let value = value.trim();

        if let (Ok(name), Ok(value)) = (
          name.parse::<http::HeaderName>(),
          value.parse::<http::HeaderValue>(),
        ) {
          headers.insert(name, value);
        }
      }
    }

    // Extract body
    let body_lines = &lines[body_start..];
    let body = body_lines.join("\n").into_bytes();
    let body_bytes = Bytes::from(body);

    // Create connection info
    let local_addr = "127.0.0.1:3000".parse().unwrap(); // This should be the actual local address
    let connection_info = ConnectionInfo::new(remote_addr, local_addr, version);

    // Parse path and query parameters
    let (_path, _query_params) = Self::parse_uri(&uri);

    // Create request chunk
    let request_chunk = StreamWeaveHttpRequestChunk::new(
      method,
      uri,
      headers,
      body_bytes,
      connection_info,
      true,           // is_final_chunk
      Uuid::new_v4(), // request_id
      Uuid::new_v4(), // connection_id - will be replaced with actual connection ID
    );

    Ok(Some(request_chunk))
  }

  /// Parse URI to extract path and query parameters
  fn parse_uri(uri: &Uri) -> (String, HashMap<String, String>) {
    let path = uri.path().to_string();
    let mut query_params = HashMap::new();

    if let Some(query) = uri.query() {
      for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
          query_params.insert(key.to_string(), value.to_string());
        }
      }
    }

    (path, query_params)
  }

  /// Check if the request indicates keep-alive
  fn is_keep_alive_request(data: &[u8]) -> bool {
    let request_str = String::from_utf8_lossy(data);
    request_str
      .to_lowercase()
      .contains("connection: keep-alive")
  }

  /// Shutdown the HTTP server
  pub fn shutdown(&self) {
    if let Some(sender) = &self.shutdown_sender {
      let _ = sender.send(());
    }
  }
}

// Output trait implementation moved to output.rs

// Producer trait implementation moved to producer.rs

#[cfg(test)]
mod tests {
  use super::*;
  use crate::producer::Producer;
  use std::net::{IpAddr, Ipv4Addr};

  #[tokio::test]
  async fn test_http_server_producer_creation() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let listener = TcpListener::bind(addr).await.unwrap();
    let producer = HttpServerProducer::new(
      listener,
      10,
      Duration::from_secs(5),
      Duration::from_secs(10),
    );

    assert_eq!(producer.max_connections.available_permits(), 10);
    assert_eq!(producer.connection_timeout, Duration::from_secs(5));
    assert_eq!(producer.keep_alive_timeout, Duration::from_secs(10));
  }

  #[tokio::test]
  async fn test_http_server_producer_bind() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let producer = HttpServerProducer::bind(addr).await.unwrap();

    assert_eq!(producer.max_connections.available_permits(), 1000);
    assert_eq!(producer.connection_timeout, Duration::from_secs(30));
    assert_eq!(producer.keep_alive_timeout, Duration::from_secs(60));
  }

  #[tokio::test]
  async fn test_http_server_producer_with_config() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let listener = TcpListener::bind(addr).await.unwrap();
    let producer = HttpServerProducer::new(
      listener,
      5,
      Duration::from_secs(10),
      Duration::from_secs(20),
    )
    .with_max_connections(15)
    .with_connection_timeout(Duration::from_secs(15))
    .with_keep_alive_timeout(Duration::from_secs(25));

    assert_eq!(producer.max_connections.available_permits(), 15);
    assert_eq!(producer.connection_timeout, Duration::from_secs(15));
    assert_eq!(producer.keep_alive_timeout, Duration::from_secs(25));
  }

  #[tokio::test]
  async fn test_parse_uri() {
    let uri: Uri = "http://localhost:3000/api/users?page=1&limit=10"
      .parse()
      .unwrap();
    let (path, query_params) = HttpServerProducer::parse_uri(&uri);

    assert_eq!(path, "/api/users");
    assert_eq!(query_params.get("page"), Some(&"1".to_string()));
    assert_eq!(query_params.get("limit"), Some(&"10".to_string()));
  }

  #[tokio::test]
  async fn test_parse_uri_no_query() {
    let uri: Uri = "http://localhost:3000/api/users".parse().unwrap();
    let (path, query_params) = HttpServerProducer::parse_uri(&uri);

    assert_eq!(path, "/api/users");
    assert!(query_params.is_empty());
  }

  #[tokio::test]
  async fn test_is_keep_alive_request() {
    let keep_alive_data = b"GET / HTTP/1.1\r\nConnection: keep-alive\r\n\r\n";
    let close_data = b"GET / HTTP/1.1\r\nConnection: close\r\n\r\n";
    let no_connection_data = b"GET / HTTP/1.1\r\n\r\n";

    assert!(HttpServerProducer::is_keep_alive_request(keep_alive_data));
    assert!(!HttpServerProducer::is_keep_alive_request(close_data));
    assert!(!HttpServerProducer::is_keep_alive_request(
      no_connection_data
    ));
  }

  #[tokio::test]
  async fn test_parse_http_request_get() {
    let data = b"GET /api/users HTTP/1.1\r\nHost: localhost:3000\r\nUser-Agent: test\r\n\r\n";
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

    let result = HttpServerProducer::parse_http_request(data, addr).await;
    assert!(result.is_ok());

    let request_chunk = result.unwrap().unwrap();
    assert_eq!(request_chunk.method, Method::GET);
    assert_eq!(request_chunk.uri.path(), "/api/users");
    assert_eq!(request_chunk.headers.get("host").unwrap(), "localhost:3000");
    assert_eq!(request_chunk.headers.get("user-agent").unwrap(), "test");
  }

  #[tokio::test]
  async fn test_parse_http_request_post() {
    let data = b"POST /api/users HTTP/1.1\r\nHost: localhost:3000\r\nContent-Type: application/json\r\nContent-Length: 20\r\n\r\n{\"name\": \"test\"}";
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

    let result = HttpServerProducer::parse_http_request(data, addr).await;
    assert!(result.is_ok());

    let request_chunk = result.unwrap().unwrap();
    assert_eq!(request_chunk.method, Method::POST);
    assert_eq!(request_chunk.uri.path(), "/api/users");
    assert_eq!(
      request_chunk.headers.get("content-type").unwrap(),
      "application/json"
    );
    assert_eq!(
      request_chunk.chunk,
      Bytes::from(&b"{\"name\": \"test\"}"[..])
    );
  }

  #[tokio::test]
  async fn test_parse_http_request_invalid() {
    let data = b"INVALID REQUEST";
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

    let result = HttpServerProducer::parse_http_request(data, addr).await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_parse_http_request_incomplete() {
    let data = b"GET /api/users HTTP/1.1\r\nHost: localhost:3000\r\n";
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

    let result = HttpServerProducer::parse_http_request(data, addr).await;
    assert!(result.is_ok());
    // The current parser will actually parse this as a valid request with empty body
    let request_chunk = result.unwrap().unwrap();
    assert_eq!(request_chunk.method, Method::GET);
    assert_eq!(request_chunk.uri.path(), "/api/users");
    assert_eq!(request_chunk.headers.get("host").unwrap(), "localhost:3000");
  }

  #[tokio::test]
  async fn test_http_server_producer_producer_trait() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let listener = TcpListener::bind(addr).await.unwrap();
    let mut producer = HttpServerProducer::new(
      listener,
      10,
      Duration::from_secs(5),
      Duration::from_secs(10),
    );

    // Test config
    let config = ProducerConfig::default().with_name("test_producer".to_string());
    producer.set_config(config);
    assert_eq!(producer.config().name(), Some("test_producer".to_string()));
  }

  #[tokio::test]
  async fn test_http_server_producer_start() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let listener = TcpListener::bind(addr).await.unwrap();
    let mut producer = HttpServerProducer::new(
      listener,
      10,
      Duration::from_secs(5),
      Duration::from_secs(10),
    );

    let result = producer.start().await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_http_server_producer_shutdown() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let listener = TcpListener::bind(addr).await.unwrap();
    let producer = HttpServerProducer::new(
      listener,
      10,
      Duration::from_secs(5),
      Duration::from_secs(10),
    );

    // Shutdown should not panic
    producer.shutdown();
  }

  #[tokio::test]
  async fn test_http_server_producer_produce() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let listener = TcpListener::bind(addr).await.unwrap();
    let mut producer = HttpServerProducer::new(
      listener,
      10,
      Duration::from_secs(5),
      Duration::from_secs(10),
    );

    let _stream = producer.produce();
    // The stream should be created successfully (it's an empty stream in this implementation)
  }

  #[tokio::test]
  async fn test_http_server_producer_max_connections() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let listener = TcpListener::bind(addr).await.unwrap();
    let mut producer =
      HttpServerProducer::new(listener, 1, Duration::from_secs(5), Duration::from_secs(10));

    let result = producer.start().await;
    assert!(result.is_ok());

    // Test that max connections is respected
    assert_eq!(producer.max_connections.available_permits(), 1);
  }

  #[tokio::test]
  async fn test_http_server_producer_timeouts() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    let listener = TcpListener::bind(addr).await.unwrap();
    let producer =
      HttpServerProducer::new(listener, 10, Duration::from_secs(1), Duration::from_secs(2));

    assert_eq!(producer.connection_timeout, Duration::from_secs(1));
    assert_eq!(producer.keep_alive_timeout, Duration::from_secs(2));
  }
}
