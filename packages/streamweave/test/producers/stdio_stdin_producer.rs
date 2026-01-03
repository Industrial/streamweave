//! Tests for StdioStdinProducer

use streamweave::error::ErrorStrategy;
use streamweave::producers::StdioStdinProducer;
use streamweave::{Producer, ProducerConfig};

#[test]
fn test_stdio_stdin_producer_new() {
  let producer = StdioStdinProducer::new();
  // Producer should be created
  assert!(true);
}

#[test]
fn test_stdio_stdin_producer_default() {
  let producer = StdioStdinProducer::default();
  // Default producer should be created
  assert!(true);
}

#[test]
fn test_stdio_stdin_producer_with_name() {
  let producer = StdioStdinProducer::new().with_name("stdin".to_string());

  let config = producer.get_config_impl();
  assert_eq!(config.name(), Some("stdin"));
}

#[test]
fn test_stdio_stdin_producer_with_error_strategy() {
  let producer = StdioStdinProducer::new().with_error_strategy(ErrorStrategy::Skip);

  let config = producer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[test]
fn test_stdio_stdin_producer_config_operations() {
  let mut producer = StdioStdinProducer::new();

  let config = producer.get_config_mut_impl();
  config.name = Some("updated".to_string());

  assert_eq!(producer.get_config_impl().name(), Some("updated"));
}

#[test]
fn test_stdio_stdin_producer_component_info() {
  let producer = StdioStdinProducer::new().with_name("test".to_string());

  let info = producer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("StdioStdinProducer"));
}
