//! Tests for ProcessConsumer

use streamweave_error::ErrorStrategy;
use streamweave_process::ProcessConsumer;

#[test]
fn test_process_consumer_new() {
  let consumer = ProcessConsumer::new("echo".to_string());
  assert_eq!(consumer.command, "echo");
  assert!(consumer.args.is_empty());
}

#[test]
fn test_process_consumer_with_args() {
  let consumer = ProcessConsumer::new("grep".to_string()).arg("pattern".to_string());
  assert_eq!(consumer.command, "grep");
  assert_eq!(consumer.args.len(), 1);
  assert_eq!(consumer.args[0], "pattern");
}

#[test]
fn test_process_consumer_with_name() {
  let consumer = ProcessConsumer::new("echo".to_string()).with_name("test_consumer".to_string());
  assert_eq!(consumer.config.name, "test_consumer");
}

#[test]
fn test_process_consumer_with_error_strategy() {
  let consumer = ProcessConsumer::new("echo".to_string()).with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    consumer.config.error_strategy,
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_process_consumer_consume_basic() {
  use futures::StreamExt;

  // Use cat to read from stdin and output
  let mut consumer = ProcessConsumer::new("cat".to_string());

  let input = futures::stream::iter(vec![
    "line1".to_string(),
    "line2".to_string(),
    "line3".to_string(),
  ]);
  let input_stream = Box::pin(input);

  consumer.consume(input_stream).await;
  // If consume completes without error, test passes
}

#[tokio::test]
async fn test_process_consumer_consume_empty_stream() {
  use futures::StreamExt;

  let mut consumer = ProcessConsumer::new("cat".to_string());

  let input = futures::stream::empty::<String>();
  let input_stream = Box::pin(input);

  consumer.consume(input_stream).await;
  // Should handle empty stream gracefully
}

#[tokio::test]
async fn test_process_consumer_consume_invalid_command() {
  use futures::StreamExt;

  // Use a command that doesn't exist
  let mut consumer = ProcessConsumer::new("nonexistent_command_xyz123".to_string());

  let input = futures::stream::iter(vec!["test".to_string()]);
  let input_stream = Box::pin(input);

  // Should handle spawn failure gracefully (logs warning and returns)
  consumer.consume(input_stream).await;
}

#[tokio::test]
async fn test_process_consumer_consume_with_args() {
  use futures::StreamExt;

  // Use grep to filter input
  let mut consumer = ProcessConsumer::new("grep".to_string()).arg("test".to_string());

  let input = futures::stream::iter(vec![
    "test line".to_string(),
    "other line".to_string(),
    "test again".to_string(),
  ]);
  let input_stream = Box::pin(input);

  consumer.consume(input_stream).await;
}

#[test]
fn test_process_consumer_config_methods() {
  use streamweave::ConsumerConfig;

  let mut consumer = ProcessConsumer::new("cat".to_string());
  let new_config = ConsumerConfig::default();

  consumer.set_config_impl(new_config.clone());
  assert_eq!(consumer.get_config_impl(), &new_config);
  assert_eq!(consumer.get_config_mut_impl(), &mut new_config);
}

#[test]
fn test_process_consumer_handle_error_stop() {
  use std::error::Error;
  use std::fmt;
  use streamweave_error::{ComponentInfo, ErrorContext, StreamError};

  let consumer = ProcessConsumer::new("cat".to_string()).with_error_strategy(ErrorStrategy::Stop);

  let error = StreamError {
    source: Box::new(StringError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some("test".to_string()),
      component_name: "test".to_string(),
      component_type: "test".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "test".to_string(),
    },
    retries: 0,
  };

  assert!(matches!(
    consumer.handle_error(&error),
    streamweave_error::ErrorAction::Stop
  ));
}

#[test]
fn test_process_consumer_handle_error_skip() {
  use streamweave_error::{ComponentInfo, ErrorContext, StreamError};

  let consumer = ProcessConsumer::new("cat".to_string()).with_error_strategy(ErrorStrategy::Skip);

  let error = StreamError {
    source: Box::new(StringError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some("test".to_string()),
      component_name: "test".to_string(),
      component_type: "test".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "test".to_string(),
    },
    retries: 0,
  };

  assert!(matches!(
    consumer.handle_error(&error),
    streamweave_error::ErrorAction::Skip
  ));
}

#[test]
fn test_process_consumer_handle_error_retry() {
  use streamweave_error::{ComponentInfo, ErrorContext, StreamError};

  let consumer =
    ProcessConsumer::new("cat".to_string()).with_error_strategy(ErrorStrategy::Retry(3));

  let error = StreamError {
    source: Box::new(StringError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some("test".to_string()),
      component_name: "test".to_string(),
      component_type: "test".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "test".to_string(),
    },
    retries: 1,
  };

  assert!(matches!(
    consumer.handle_error(&error),
    streamweave_error::ErrorAction::Retry
  ));
}

#[test]
fn test_process_consumer_handle_error_retry_exceeded() {
  use streamweave_error::{ComponentInfo, ErrorContext, StreamError};

  let consumer =
    ProcessConsumer::new("cat".to_string()).with_error_strategy(ErrorStrategy::Retry(3));

  let error = StreamError {
    source: Box::new(StringError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some("test".to_string()),
      component_name: "test".to_string(),
      component_type: "test".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "test".to_string(),
    },
    retries: 3,
  };

  assert!(matches!(
    consumer.handle_error(&error),
    streamweave_error::ErrorAction::Stop
  ));
}

#[test]
fn test_process_consumer_handle_error_custom() {
  use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, StreamError};

  let consumer = ProcessConsumer::new("cat".to_string())
    .with_error_strategy(ErrorStrategy::new_custom(|_| ErrorAction::Skip));

  let error = StreamError {
    source: Box::new(StringError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some("test".to_string()),
      component_name: "test".to_string(),
      component_type: "test".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "test".to_string(),
    },
    retries: 0,
  };

  assert!(matches!(
    consumer.handle_error(&error),
    streamweave_error::ErrorAction::Skip
  ));
}

#[test]
fn test_process_consumer_create_error_context() {
  let consumer = ProcessConsumer::new("cat".to_string()).with_name("test-consumer".to_string());
  let context = consumer.create_error_context(Some("test_item".to_string()));

  assert_eq!(context.component_name, "test-consumer");
  assert_eq!(context.item, Some("test_item".to_string()));
  assert!(context.component_type.contains("ProcessConsumer"));
}

#[test]
fn test_process_consumer_create_error_context_default_name() {
  let consumer = ProcessConsumer::new("cat".to_string());
  let context = consumer.create_error_context(None);

  // When name is not set, it uses the default from config
  assert_eq!(context.component_name, "");
  assert_eq!(context.item, None);
}

#[test]
fn test_process_consumer_component_info() {
  let consumer = ProcessConsumer::new("cat".to_string()).with_name("test-consumer".to_string());
  let info = consumer.component_info();

  assert_eq!(info.name, "test-consumer");
  assert!(info.type_name.contains("ProcessConsumer"));
}

#[test]
fn test_process_consumer_component_info_default_name() {
  let consumer = ProcessConsumer::new("cat".to_string());
  let info = consumer.component_info();

  // When name is not set, it uses empty string from config
  assert_eq!(info.name, "");
}

#[test]
fn test_process_consumer_multiple_args() {
  let consumer = ProcessConsumer::new("grep".to_string())
    .arg("-i".to_string())
    .arg("pattern".to_string());

  assert_eq!(consumer.args.len(), 2);
  assert_eq!(consumer.args[0], "-i");
  assert_eq!(consumer.args[1], "pattern");
}

#[test]
fn test_process_consumer_input_trait() {
  use streamweave::Input;

  let consumer = ProcessConsumer::new("cat".to_string());
  // Verify Input trait is implemented
  assert_eq!(
    std::any::type_name::<ProcessConsumer::Input>(),
    std::any::type_name::<String>()
  );
}

// Helper for error tests
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct StringError(String);

impl fmt::Display for StringError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl Error for StringError {}
