//! Tests for StdioStdoutConsumer

use streamweave::consumers::StdioStdoutConsumer;
use streamweave::error::ErrorStrategy;
use streamweave::{Consumer, ConsumerConfig};

#[test]
fn test_stdio_stdout_consumer_new() {
  let consumer = StdioStdoutConsumer::<String>::new();
  // Consumer should be created
  assert!(true);
}

#[test]
fn test_stdio_stdout_consumer_default() {
  let consumer = StdioStdoutConsumer::<String>::default();
  // Default consumer should be created
  assert!(true);
}

#[test]
fn test_stdio_stdout_consumer_with_name() {
  let consumer = StdioStdoutConsumer::<String>::new().with_name("stdout".to_string());

  assert_eq!(consumer.config.name, "stdout");
}

#[test]
fn test_stdio_stdout_consumer_with_error_strategy() {
  let consumer = StdioStdoutConsumer::<String>::new().with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(
    consumer.config.error_strategy,
    ErrorStrategy::Skip
  ));
}

#[test]
fn test_stdio_stdout_consumer_config_operations() {
  let mut consumer = StdioStdoutConsumer::<String>::new();

  let mut config = ConsumerConfig::default();
  config.name = "updated".to_string();
  consumer.set_config_impl(config);

  assert_eq!(consumer.get_config_impl().name, "updated");
}

#[test]
fn test_stdio_stdout_consumer_component_info() {
  let consumer = StdioStdoutConsumer::<String>::new().with_name("test".to_string());

  let info = consumer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("StdioStdoutConsumer"));
}
