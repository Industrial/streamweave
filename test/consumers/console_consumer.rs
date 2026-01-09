//! Tests for ConsoleConsumer

use futures::stream;
use streamweave::consumers::ConsoleConsumer;
use streamweave::error::{ErrorAction, ErrorStrategy, StreamError};
use streamweave::{Consumer, Input};

#[tokio::test]
async fn test_console_consumer_new() {
  let consumer = ConsoleConsumer::<i32>::new();
  assert_eq!(consumer.config.name, "");
  assert!(matches!(
    consumer.config.error_strategy,
    ErrorStrategy::Stop
  ));
}

#[tokio::test]
async fn test_console_consumer_default() {
  let consumer = ConsoleConsumer::<i32>::default();
  assert_eq!(consumer.config.name, "");
}

#[tokio::test]
async fn test_console_consumer_with_name() {
  let consumer = ConsoleConsumer::<i32>::new().with_name("test-console".to_string());
  assert_eq!(consumer.config.name, "test-console");
}

#[tokio::test]
async fn test_console_consumer_with_error_strategy() {
  let consumer = ConsoleConsumer::<i32>::new().with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    consumer.config.error_strategy,
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_console_consumer_consume_basic() {
  let mut consumer = ConsoleConsumer::<i32>::new();
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

  // This will print to stdout, but we can't easily test that
  // So we just verify it doesn't panic
  consumer.consume(input_stream).await;
}

#[tokio::test]
async fn test_console_consumer_consume_empty_stream() {
  let mut consumer = ConsoleConsumer::<i32>::new();
  let input_stream = Box::pin(stream::empty::<i32>());

  consumer.consume(input_stream).await;
  // Should handle empty stream gracefully
}

#[tokio::test]
async fn test_console_consumer_consume_strings() {
  let mut consumer = ConsoleConsumer::<String>::new();
  let input_stream = Box::pin(stream::iter(vec![
    "hello".to_string(),
    "world".to_string(),
    "test".to_string(),
  ]));

  consumer.consume(input_stream).await;
}

#[tokio::test]
async fn test_console_consumer_error_handling_stop() {
  let consumer = ConsoleConsumer::<i32>::new().with_error_strategy(ErrorStrategy::Stop);
  let error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    0,
    consumer.create_error_context(Some(42)),
    consumer.component_info(),
  );

  assert_eq!(consumer.handle_error(&error), ErrorAction::Stop);
}

#[tokio::test]
async fn test_console_consumer_error_handling_skip() {
  let consumer = ConsoleConsumer::<i32>::new().with_error_strategy(ErrorStrategy::Skip);
  let error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    0,
    consumer.create_error_context(Some(42)),
    consumer.component_info(),
  );

  assert_eq!(consumer.handle_error(&error), ErrorAction::Skip);
}

#[tokio::test]
async fn test_console_consumer_error_handling_retry() {
  let consumer = ConsoleConsumer::<i32>::new().with_error_strategy(ErrorStrategy::Retry(3));
  let error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    2,
    consumer.create_error_context(Some(42)),
    consumer.component_info(),
  );

  assert_eq!(consumer.handle_error(&error), ErrorAction::Retry);
}

#[tokio::test]
async fn test_console_consumer_error_handling_retry_exceeded() {
  let consumer = ConsoleConsumer::<i32>::new().with_error_strategy(ErrorStrategy::Retry(3));
  let error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    3,
    consumer.create_error_context(Some(42)),
    consumer.component_info(),
  );

  assert_eq!(consumer.handle_error(&error), ErrorAction::Stop);
}

#[tokio::test]
async fn test_console_consumer_create_error_context() {
  let consumer = ConsoleConsumer::<i32>::new().with_name("test-console".to_string());
  let context = consumer.create_error_context(Some(42));

  assert_eq!(context.item, Some(42));
  assert_eq!(context.component_name, "test-console");
  assert!(context.component_type.contains("ConsoleConsumer"));
}

#[tokio::test]
async fn test_console_consumer_component_info() {
  let consumer = ConsoleConsumer::<i32>::new().with_name("test-console".to_string());
  let info = consumer.component_info();

  assert_eq!(info.name, "test-console");
  assert!(info.type_name.contains("ConsoleConsumer"));
}

#[tokio::test]
async fn test_console_consumer_component_info_default_name() {
  let consumer = ConsoleConsumer::<i32>::new();
  let info = consumer.component_info();

  assert_eq!(info.name, "");
  assert!(info.type_name.contains("ConsoleConsumer"));
}

#[tokio::test]
async fn test_console_consumer_config_access() {
  let mut consumer = ConsoleConsumer::<i32>::new().with_name("test".to_string());
  let config = consumer.config();
  assert_eq!(config.name, "test");

  let config_mut = consumer.config_mut();
  config_mut.name = "updated".to_string();
  assert_eq!(consumer.config().name, "updated");
}

#[tokio::test]
async fn test_console_consumer_input_trait() {
  let consumer = ConsoleConsumer::<i32>::new();
  // Verify Input trait is implemented
  let _input_type: <ConsoleConsumer<i32> as Input>::Input = 42;
  // Should compile - this is a compile-time check
  assert!(true);
}
