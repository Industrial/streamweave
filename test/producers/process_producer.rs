//! Tests for ProcessProducer

use streamweave::error::ErrorStrategy;
use streamweave::producers::ProcessProducer;

#[test]
fn test_process_producer_new() {
  let producer = ProcessProducer::new("echo".to_string());
  assert_eq!(producer.command, "echo");
  assert!(producer.args.is_empty());
}

#[test]
fn test_process_producer_with_args() {
  let producer = ProcessProducer::new("ls".to_string()).arg("-la".to_string());
  assert_eq!(producer.command, "ls");
  assert_eq!(producer.args.len(), 1);
  assert_eq!(producer.args[0], "-la");
}

#[test]
fn test_process_producer_with_name() {
  let producer = ProcessProducer::new("echo".to_string()).with_name("test_producer".to_string());
  assert_eq!(producer.config.name(), Some("test_producer"));
}

#[test]
fn test_process_producer_with_error_strategy() {
  let producer = ProcessProducer::new("echo".to_string()).with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    producer.config.error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_process_producer_produce_basic() {
  use futures::StreamExt;

  let mut producer = ProcessProducer::new("echo".to_string())
    .arg("hello".to_string())
    .arg("world".to_string());

  let mut stream = producer.produce();
  let mut results = Vec::new();

  while let Some(item) = stream.next().await {
    results.push(item);
  }

  // echo outputs "hello world" on a single line
  assert!(!results.is_empty());
  assert!(
    results
      .iter()
      .any(|s| s.contains("hello") || s.contains("world"))
  );
}

#[tokio::test]
async fn test_process_producer_produce_multiple_lines() {
  use futures::StreamExt;

  // Use printf to create multiple lines
  let mut producer = ProcessProducer::new("printf".to_string())
    .arg("%s\n%s\n%s".to_string())
    .arg("line1".to_string())
    .arg("line2".to_string())
    .arg("line3".to_string());

  let mut stream = producer.produce();
  let mut results = Vec::new();

  while let Some(item) = stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 3);
  assert!(results.iter().any(|s| s.contains("line1")));
  assert!(results.iter().any(|s| s.contains("line2")));
  assert!(results.iter().any(|s| s.contains("line3")));
}

#[tokio::test]
async fn test_process_producer_produce_empty_output() {
  use futures::StreamExt;

  // Use a command that produces no output
  let mut producer = ProcessProducer::new("true".to_string());

  let mut stream = producer.produce();
  let mut results = Vec::new();

  while let Some(item) = stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_process_producer_produce_invalid_command() {
  use futures::StreamExt;

  // Use a command that doesn't exist
  let mut producer = ProcessProducer::new("nonexistent_command_xyz123".to_string());

  let mut stream = producer.produce();
  let mut results = Vec::new();

  // Should handle gracefully and produce empty stream
  while let Some(item) = stream.next().await {
    results.push(item);
  }

  // Should produce no items (command failed to spawn)
  assert_eq!(results.len(), 0);
}

#[test]
fn test_process_producer_config_methods() {
  use streamweave::ProducerConfig;

  let mut producer = ProcessProducer::new("echo".to_string());
  let new_config = ProducerConfig::default();

  producer.set_config_impl(new_config.clone());
  assert_eq!(producer.get_config_impl(), &new_config);
  assert_eq!(producer.get_config_mut_impl(), &mut new_config);
}

#[test]
fn test_process_producer_handle_error_stop() {
  use std::error::Error;
  use std::fmt;
  use streamweave::error::{ComponentInfo, ErrorContext, StreamError};

  let producer = ProcessProducer::new("echo".to_string()).with_error_strategy(ErrorStrategy::Stop);

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
    producer.handle_error(&error),
    streamweave::error::ErrorAction::Stop
  ));
}

#[test]
fn test_process_producer_handle_error_skip() {
  use streamweave::error::{ComponentInfo, ErrorContext, StreamError};

  let producer = ProcessProducer::new("echo".to_string()).with_error_strategy(ErrorStrategy::Skip);

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
    producer.handle_error(&error),
    streamweave::error::ErrorAction::Skip
  ));
}

#[test]
fn test_process_producer_handle_error_retry() {
  use streamweave::error::{ComponentInfo, ErrorContext, StreamError};

  let producer =
    ProcessProducer::new("echo".to_string()).with_error_strategy(ErrorStrategy::Retry(3));

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
    producer.handle_error(&error),
    streamweave::error::ErrorAction::Retry
  ));
}

#[test]
fn test_process_producer_handle_error_retry_exceeded() {
  use streamweave::error::{ComponentInfo, ErrorContext, StreamError};

  let producer =
    ProcessProducer::new("echo".to_string()).with_error_strategy(ErrorStrategy::Retry(3));

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
    producer.handle_error(&error),
    streamweave::error::ErrorAction::Stop
  ));
}

#[test]
fn test_process_producer_handle_error_custom() {
  use streamweave::error::{ComponentInfo, ErrorAction, ErrorContext, StreamError};

  let producer = ProcessProducer::new("echo".to_string())
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
    producer.handle_error(&error),
    streamweave::error::ErrorAction::Skip
  ));
}

#[test]
fn test_process_producer_create_error_context() {
  let producer = ProcessProducer::new("echo".to_string()).with_name("test-producer".to_string());
  let context = producer.create_error_context(Some("test_item".to_string()));

  assert_eq!(context.component_name, "test-producer");
  assert_eq!(context.item, Some("test_item".to_string()));
  assert!(context.component_type.contains("ProcessProducer"));
}

#[test]
fn test_process_producer_create_error_context_default_name() {
  let producer = ProcessProducer::new("echo".to_string());
  let context = producer.create_error_context(None);

  assert_eq!(context.component_name, "process_producer");
  assert_eq!(context.item, None);
}

#[test]
fn test_process_producer_component_info() {
  let producer = ProcessProducer::new("echo".to_string()).with_name("test-producer".to_string());
  let info = producer.component_info();

  assert_eq!(info.name, "test-producer");
  assert!(info.type_name.contains("ProcessProducer"));
}

#[test]
fn test_process_producer_component_info_default_name() {
  let producer = ProcessProducer::new("echo".to_string());
  let info = producer.component_info();

  assert_eq!(info.name, "process_producer");
}

#[test]
fn test_process_producer_multiple_args() {
  let producer = ProcessProducer::new("ls".to_string())
    .arg("-la".to_string())
    .arg("/tmp".to_string());

  assert_eq!(producer.args.len(), 2);
  assert_eq!(producer.args[0], "-la");
  assert_eq!(producer.args[1], "/tmp");
}

#[test]
fn test_process_producer_output_trait() {
  use streamweave::Output;

  let producer = ProcessProducer::new("echo".to_string());
  // Verify Output trait is implemented
  assert_eq!(
    std::any::type_name::<ProcessProducer::Output>(),
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
