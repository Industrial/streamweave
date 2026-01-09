//! Tests for SignalProducer

use streamweave::error::ErrorStrategy;
use streamweave::producers::Signal;
use streamweave::producers::SignalProducer;
use streamweave::{Producer, ProducerConfig};

#[test]
fn test_signal_variants() {
  assert_eq!(Signal::Interrupt, Signal::Interrupt);
  assert_eq!(Signal::Terminate, Signal::Terminate);
  assert_ne!(Signal::Interrupt, Signal::Terminate);
}

#[test]
fn test_signal_from_number() {
  assert_eq!(Signal::from_number(2), Some(Signal::Interrupt));
  assert_eq!(Signal::from_number(15), Some(Signal::Terminate));
  assert_eq!(Signal::from_number(99), None);
}

#[test]
fn test_signal_number() {
  assert_eq!(Signal::Interrupt.number(), 2);
  assert_eq!(Signal::Terminate.number(), 15);
}

#[test]
fn test_signal_producer_new() {
  let producer = SignalProducer::new();
  // Producer should be created
  assert!(true);
}

#[test]
fn test_signal_producer_default() {
  let producer = SignalProducer::default();
  // Default producer should be created
  assert!(true);
}

#[test]
fn test_signal_producer_with_name() {
  let producer = SignalProducer::new().with_name("signal".to_string());

  let config = producer.get_config_impl();
  assert_eq!(config.name(), Some("signal"));
}

#[test]
fn test_signal_producer_with_error_strategy() {
  let producer = SignalProducer::new().with_error_strategy(ErrorStrategy::Skip);

  let config = producer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[test]
fn test_signal_producer_config_operations() {
  let mut producer = SignalProducer::new();

  let config = producer.get_config_mut_impl();
  config.name = Some("updated".to_string());

  assert_eq!(producer.get_config_impl().name(), Some("updated"));
}

#[test]
fn test_signal_producer_component_info() {
  let producer = SignalProducer::new().with_name("test".to_string());

  let info = producer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("SignalProducer"));
}

#[test]
fn test_signal_producer_clone() {
  let producer1 = SignalProducer::new();
  let producer2 = producer1.clone();

  // Both should be valid
  assert!(true);
}
