//! Tests for StdinProducer

use streamweave::Producer;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_stdio::StdinProducer;

#[test]
fn test_stdin_producer_new() {
  let producer = StdinProducer::new();
  assert_eq!(producer.config().name(), None);
}

#[test]
fn test_stdin_producer_builder() {
  let producer = StdinProducer::new()
    .with_error_strategy(ErrorStrategy::Skip)
    .with_name("test_producer".to_string());

  assert_eq!(producer.config().error_strategy(), ErrorStrategy::Skip);
  assert_eq!(producer.config().name(), Some("test_producer".to_string()));
}

#[test]
fn test_stdin_producer_default() {
  let producer = StdinProducer::default();
  assert_eq!(producer.config().name(), None);
}
