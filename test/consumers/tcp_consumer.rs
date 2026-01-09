//! Tests for TcpConsumer

use futures::stream;
use std::net::TcpListener;
use std::pin::Pin;
use std::time::Duration;
use streamweave::consumers::{TcpConsumer, TcpConsumerConfig};
use streamweave::error::{ErrorAction, ErrorStrategy, StreamError};
use streamweave::{Consumer, Input};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::time::timeout;

async fn start_test_server(port: u16) -> tokio::task::JoinHandle<()> {
  tokio::spawn(async move {
    let listener =
      TcpListener::bind(format!("127.0.0.1:{}", port)).expect("Failed to bind test server");
    listener.set_nonblocking(true).unwrap();

    let listener = tokio::net::TcpListener::from_std(listener).unwrap();
    while let Ok((mut stream, _)) = listener.accept().await {
      let mut buf = vec![0u8; 1024];
      let _ = stream.read(&mut buf).await;
    }
  })
}

#[tokio::test]
async fn test_tcp_consumer_config_default() {
  let config = TcpConsumerConfig::default();
  assert_eq!(config.address, "127.0.0.1:8080");
  assert_eq!(config.timeout_secs, 30);
  assert!(config.append_newline);
  assert!(config.delimiter.is_none());
}

#[tokio::test]
async fn test_tcp_consumer_config_with_address() {
  let config = TcpConsumerConfig::default().with_address("192.168.1.1:9000");
  assert_eq!(config.address, "192.168.1.1:9000");
}

#[tokio::test]
async fn test_tcp_consumer_config_with_timeout() {
  let config = TcpConsumerConfig::default().with_timeout_secs(60);
  assert_eq!(config.timeout_secs, 60);
}

#[tokio::test]
async fn test_tcp_consumer_config_with_append_newline() {
  let config = TcpConsumerConfig::default().with_append_newline(false);
  assert!(!config.append_newline);
}

#[tokio::test]
async fn test_tcp_consumer_config_with_delimiter() {
  let config = TcpConsumerConfig::default().with_delimiter(Some(vec![0xFF, 0xFE]));
  assert_eq!(config.delimiter, Some(vec![0xFF, 0xFE]));
}

#[tokio::test]
async fn test_tcp_consumer_new() {
  let config = TcpConsumerConfig::default().with_address("127.0.0.1:8080");
  let consumer = TcpConsumer::new(config.clone());
  assert_eq!(consumer.tcp_config.address, "127.0.0.1:8080");
  assert_eq!(consumer.config.name, "");
}

#[tokio::test]
async fn test_tcp_consumer_with_name() {
  let config = TcpConsumerConfig::default();
  let consumer = TcpConsumer::new(config).with_name("test-tcp".to_string());
  assert_eq!(consumer.config.name, "test-tcp");
}

#[tokio::test]
async fn test_tcp_consumer_with_error_strategy() {
  let config = TcpConsumerConfig::default();
  let consumer = TcpConsumer::new(config).with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    consumer.config.error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_tcp_consumer_consume_basic() {
  // Start a test server
  let port = 18080;
  let _server = start_test_server(port).await;

  // Give server time to start
  tokio::time::sleep(Duration::from_millis(100)).await;

  let config = TcpConsumerConfig::default()
    .with_address(format!("127.0.0.1:{}", port))
    .with_timeout_secs(5);
  let mut consumer = TcpConsumer::new(config);
  let input_stream = Box::pin(stream::iter(vec!["hello".to_string(), "world".to_string()]));

  // This should connect and send data
  consumer.consume(input_stream).await;
}

#[tokio::test]
async fn test_tcp_consumer_consume_empty_stream() {
  let port = 18081;
  let _server = start_test_server(port).await;
  tokio::time::sleep(Duration::from_millis(100)).await;

  let config = TcpConsumerConfig::default()
    .with_address(format!("127.0.0.1:{}", port))
    .with_timeout_secs(5);
  let mut consumer = TcpConsumer::new(config);
  let input_stream = Box::pin(stream::empty::<String>());

  consumer.consume(input_stream).await;
  // Should handle empty stream gracefully
}

#[tokio::test]
async fn test_tcp_consumer_consume_with_delimiter() {
  let port = 18082;
  let _server = start_test_server(port).await;
  tokio::time::sleep(Duration::from_millis(100)).await;

  let config = TcpConsumerConfig::default()
    .with_address(format!("127.0.0.1:{}", port))
    .with_timeout_secs(5)
    .with_append_newline(false)
    .with_delimiter(Some(vec![0x00]));
  let mut consumer = TcpConsumer::new(config);
  let input_stream = Box::pin(stream::iter(vec!["test".to_string()]));

  consumer.consume(input_stream).await;
}

#[tokio::test]
async fn test_tcp_consumer_invalid_address() {
  let config = TcpConsumerConfig::default().with_address("invalid-address");
  let mut consumer = TcpConsumer::new(config);
  let input_stream = Box::pin(stream::iter(vec!["test".to_string()]));

  // Should handle invalid address gracefully
  consumer.consume(input_stream).await;
}

#[tokio::test]
async fn test_tcp_consumer_connection_timeout() {
  let config = TcpConsumerConfig::default()
    .with_address("127.0.0.1:19999") // Port that won't be listening
    .with_timeout_secs(1);
  let mut consumer = TcpConsumer::new(config);
  let input_stream = Box::pin(stream::iter(vec!["test".to_string()]));

  // Should handle timeout gracefully
  consumer.consume(input_stream).await;
}

#[tokio::test]
async fn test_tcp_consumer_error_handling_stop() {
  let config = TcpConsumerConfig::default();
  let consumer = TcpConsumer::new(config).with_error_strategy(ErrorStrategy::Stop);
  let error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    0,
    consumer.create_error_context(Some("test".to_string())),
    consumer.component_info(),
  );

  assert_eq!(consumer.handle_error(&error), ErrorAction::Stop);
}

#[tokio::test]
async fn test_tcp_consumer_error_handling_skip() {
  let config = TcpConsumerConfig::default();
  let consumer = TcpConsumer::new(config).with_error_strategy(ErrorStrategy::Skip);
  let error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    0,
    consumer.create_error_context(Some("test".to_string())),
    consumer.component_info(),
  );

  assert_eq!(consumer.handle_error(&error), ErrorAction::Skip);
}

#[tokio::test]
async fn test_tcp_consumer_error_handling_retry() {
  let config = TcpConsumerConfig::default();
  let consumer = TcpConsumer::new(config).with_error_strategy(ErrorStrategy::Retry(3));
  let error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    2,
    consumer.create_error_context(Some("test".to_string())),
    consumer.component_info(),
  );

  assert_eq!(consumer.handle_error(&error), ErrorAction::Retry);
}

#[tokio::test]
async fn test_tcp_consumer_error_handling_retry_exceeded() {
  let config = TcpConsumerConfig::default();
  let consumer = TcpConsumer::new(config).with_error_strategy(ErrorStrategy::Retry(3));
  let error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    3,
    consumer.create_error_context(Some("test".to_string())),
    consumer.component_info(),
  );

  assert_eq!(consumer.handle_error(&error), ErrorAction::Stop);
}

#[tokio::test]
async fn test_tcp_consumer_create_error_context() {
  let config = TcpConsumerConfig::default();
  let consumer = TcpConsumer::new(config).with_name("test-tcp".to_string());
  let context = consumer.create_error_context(Some("test".to_string()));

  assert_eq!(context.item, Some("test".to_string()));
  assert_eq!(context.component_name, "test-tcp");
  assert!(context.component_type.contains("TcpConsumer"));
}

#[tokio::test]
async fn test_tcp_consumer_component_info() {
  let config = TcpConsumerConfig::default();
  let consumer = TcpConsumer::new(config).with_name("test-tcp".to_string());
  let info = consumer.component_info();

  assert_eq!(info.name, "test-tcp");
  assert!(info.type_name.contains("TcpConsumer"));
}

#[tokio::test]
async fn test_tcp_consumer_component_info_default_name() {
  let config = TcpConsumerConfig::default();
  let consumer = TcpConsumer::new(config);
  let info = consumer.component_info();

  assert_eq!(info.name, "tcp_consumer");
  assert!(info.type_name.contains("TcpConsumer"));
}

#[tokio::test]
async fn test_tcp_consumer_config_access() {
  let config = TcpConsumerConfig::default();
  let mut consumer = TcpConsumer::new(config).with_name("test".to_string());
  let config_ref = consumer.config();
  assert_eq!(config_ref.name, "test");

  let config_mut = consumer.config_mut();
  config_mut.name = "updated".to_string();
  assert_eq!(consumer.config().name, "updated");
}

#[tokio::test]
async fn test_tcp_consumer_input_trait() {
  let config = TcpConsumerConfig::default();
  let consumer = TcpConsumer::new(config);
  // Verify Input trait is implemented
  let _input_type: <TcpConsumer as Input>::Input = "test".to_string();
  // Should compile - this is a compile-time check
  assert!(true);
}

#[tokio::test]
async fn test_tcp_consumer_append_newline() {
  let port = 18083;
  let _server = start_test_server(port).await;
  tokio::time::sleep(Duration::from_millis(100)).await;

  let config = TcpConsumerConfig::default()
    .with_address(format!("127.0.0.1:{}", port))
    .with_timeout_secs(5)
    .with_append_newline(true);
  let mut consumer = TcpConsumer::new(config);
  let input_stream = Box::pin(stream::iter(vec!["test".to_string()]));

  consumer.consume(input_stream).await;
}

#[tokio::test]
async fn test_tcp_consumer_no_newline_no_delimiter() {
  let port = 18084;
  let _server = start_test_server(port).await;
  tokio::time::sleep(Duration::from_millis(100)).await;

  let config = TcpConsumerConfig::default()
    .with_address(format!("127.0.0.1:{}", port))
    .with_timeout_secs(5)
    .with_append_newline(false)
    .with_delimiter(None);
  let mut consumer = TcpConsumer::new(config);
  let input_stream = Box::pin(stream::iter(vec!["test".to_string()]));

  consumer.consume(input_stream).await;
}
