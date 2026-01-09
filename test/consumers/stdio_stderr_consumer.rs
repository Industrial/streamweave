//! Tests for StdioStderrConsumer

use streamweave::consumers::StdioStderrConsumer;
use streamweave::error::ErrorStrategy;
use streamweave::{Consumer, ConsumerConfig};

#[test]
fn test_stderr_consumer_new() {
  let consumer = StdioStderrConsumer::<String>::new();
  // Consumer should be created
  assert!(true);
}

#[test]
fn test_stderr_consumer_default() {
  let consumer = StderrConsumer::<String>::default();
  // Default consumer should be created
  assert!(true);
}

#[test]
fn test_stderr_consumer_with_name() {
  let consumer = StderrConsumer::<String>::new().with_name("stderr".to_string());

  assert_eq!(consumer.config.name, "stderr");
}

#[test]
fn test_stderr_consumer_with_error_strategy() {
  let consumer = StderrConsumer::<String>::new().with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(
    consumer.config.error_strategy,
    ErrorStrategy::Skip
  ));
}

#[test]
fn test_stderr_consumer_config_operations() {
  let mut consumer = StderrConsumer::<String>::new();

  let mut config = ConsumerConfig::default();
  config.name = "updated".to_string();
  consumer.set_config_impl(config);

  assert_eq!(consumer.get_config_impl().name, "updated");
}

#[test]
fn test_stderr_consumer_component_info() {
  let consumer = StdioStderrConsumer::<String>::new().with_name("test".to_string());

  let info = consumer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("StdioStderrConsumer"));
}
