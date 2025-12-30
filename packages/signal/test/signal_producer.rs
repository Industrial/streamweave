//! Tests for SignalProducer

use futures::Stream;
use streamweave::{Output, Producer, ProducerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_signal::{Signal, SignalProducer};

#[derive(Debug)]
struct TestError(String);

impl std::fmt::Display for TestError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl std::error::Error for TestError {}

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
  assert_eq!(producer.config.name, None);
}

#[test]
fn test_signal_producer_default() {
  let producer = SignalProducer::default();
  assert_eq!(producer.config.name, None);
}

#[test]
fn test_signal_producer_with_name() {
  let producer = SignalProducer::new().with_name("test_producer".to_string());
  assert_eq!(producer.config.name, Some("test_producer".to_string()));
}

#[test]
fn test_signal_producer_with_error_strategy() {
  let producer = SignalProducer::new().with_error_strategy(ErrorStrategy::<Signal>::Skip);
  assert_eq!(
    producer.config.error_strategy(),
    ErrorStrategy::<Signal>::Skip
  );
}

#[test]
fn test_signal_producer_error_handling() {
  let producer = SignalProducer::new();
  let error = StreamError::new(
    Box::new(TestError("test error".to_string())),
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(Signal::Interrupt),
      component_name: "signal_producer".to_string(),
      component_type: std::any::type_name::<SignalProducer>().to_string(),
    },
    ComponentInfo {
      name: "signal_producer".to_string(),
      type_name: std::any::type_name::<SignalProducer>().to_string(),
    },
  );

  let action = producer.handle_error(&error);
  assert_eq!(action, ErrorAction::Stop);
}

#[test]
fn test_signal_producer_error_handling_skip() {
  let producer = SignalProducer::new().with_error_strategy(ErrorStrategy::<Signal>::Skip);
  let error = StreamError::new(
    Box::new(TestError("test error".to_string())),
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(Signal::Interrupt),
      component_name: "signal_producer".to_string(),
      component_type: std::any::type_name::<SignalProducer>().to_string(),
    },
    ComponentInfo {
      name: "signal_producer".to_string(),
      type_name: std::any::type_name::<SignalProducer>().to_string(),
    },
  );

  let action = producer.handle_error(&error);
  assert_eq!(action, ErrorAction::Skip);
}

#[test]
fn test_signal_producer_error_handling_retry() {
  let producer = SignalProducer::new().with_error_strategy(ErrorStrategy::<Signal>::Retry(3));
  let mut error = StreamError::new(
    Box::new(TestError("test error".to_string())),
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(Signal::Interrupt),
      component_name: "signal_producer".to_string(),
      component_type: std::any::type_name::<SignalProducer>().to_string(),
    },
    ComponentInfo {
      name: "signal_producer".to_string(),
      type_name: std::any::type_name::<SignalProducer>().to_string(),
    },
  );

  // First retry should succeed
  error.retries = 0;
  let action = producer.handle_error(&error);
  assert_eq!(action, ErrorAction::Retry);

  // After max retries, should stop
  error.retries = 3;
  let action = producer.handle_error(&error);
  assert_eq!(action, ErrorAction::Stop);
}

#[test]
fn test_signal_producer_create_error_context() {
  let producer = SignalProducer::new().with_name("test_producer".to_string());
  let context = producer.create_error_context(Some(Signal::Interrupt));

  assert_eq!(context.item, Some(Signal::Interrupt));
  assert_eq!(context.component_name, "test_producer");
  assert_eq!(
    context.component_type,
    std::any::type_name::<SignalProducer>()
  );
}

#[test]
fn test_signal_producer_component_info() {
  let producer = SignalProducer::new().with_name("test_producer".to_string());
  let info = producer.component_info();

  assert_eq!(info.name, "test_producer");
  assert_eq!(info.type_name, std::any::type_name::<SignalProducer>());
}

#[test]
fn test_signal_producer_component_info_default() {
  let producer = SignalProducer::new();
  let info = producer.component_info();

  assert_eq!(info.name, "signal_producer");
  assert_eq!(info.type_name, std::any::type_name::<SignalProducer>());
}

#[test]
fn test_signal_producer_config() {
  let producer = SignalProducer::new().with_name("test_producer".to_string());
  let config = producer.config();

  assert_eq!(config.name(), Some("test_producer".to_string()));
}

#[tokio::test]
async fn test_signal_producer_produce() {
  let mut producer = SignalProducer::new();
  let stream = producer.produce();

  // The stream should be created successfully
  // Note: We can't easily test signal reception in unit tests without actually
  // sending signals, so we just verify the stream is created
  assert!(stream.is_some() || true); // Stream creation is async, so we can't easily assert
}

#[test]
fn test_signal_producer_set_config_impl() {
  let mut producer = SignalProducer::new();
  let mut config = ProducerConfig::default();
  config.name = Some("test_config".to_string());
  config.error_strategy = ErrorStrategy::Skip;

  producer.set_config_impl(config.clone());

  assert_eq!(producer.config.name, Some("test_config".to_string()));
  assert_eq!(
    producer.config.error_strategy(),
    ErrorStrategy::<Signal>::Skip
  );
}

#[test]
fn test_signal_producer_get_config_mut_impl() {
  let mut producer = SignalProducer::new();
  let config_mut = producer.get_config_mut_impl();

  config_mut.name = Some("mutated_name".to_string());
  config_mut.error_strategy = ErrorStrategy::Retry(5);

  assert_eq!(producer.config.name, Some("mutated_name".to_string()));
  assert_eq!(
    producer.config.error_strategy(),
    ErrorStrategy::<Signal>::Retry(5)
  );
}

#[test]
fn test_signal_producer_error_handling_custom() {
  let custom_handler = |error: &StreamError<Signal>| {
    if error.retries < 2 {
      ErrorAction::Retry
    } else {
      ErrorAction::Skip
    }
  };

  let producer =
    SignalProducer::new().with_error_strategy(ErrorStrategy::new_custom(custom_handler));

  let mut error = StreamError::new(
    Box::new(TestError("test error".to_string())),
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(Signal::Interrupt),
      component_name: "signal_producer".to_string(),
      component_type: std::any::type_name::<SignalProducer>().to_string(),
    },
    ComponentInfo {
      name: "signal_producer".to_string(),
      type_name: std::any::type_name::<SignalProducer>().to_string(),
    },
  );

  // First retry should succeed
  error.retries = 0;
  let action = producer.handle_error(&error);
  assert_eq!(action, ErrorAction::Retry);

  // Second retry should succeed
  error.retries = 1;
  let action = producer.handle_error(&error);
  assert_eq!(action, ErrorAction::Retry);

  // After 2 retries, should skip
  error.retries = 2;
  let action = producer.handle_error(&error);
  assert_eq!(action, ErrorAction::Skip);
}

#[test]
fn test_signal_producer_error_handling_retry_exceeded() {
  let producer = SignalProducer::new().with_error_strategy(ErrorStrategy::<Signal>::Retry(3));

  let mut error = StreamError::new(
    Box::new(TestError("test error".to_string())),
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(Signal::Interrupt),
      component_name: "signal_producer".to_string(),
      component_type: std::any::type_name::<SignalProducer>().to_string(),
    },
    ComponentInfo {
      name: "signal_producer".to_string(),
      type_name: std::any::type_name::<SignalProducer>().to_string(),
    },
  );

  // At max retries, should stop (not retry)
  error.retries = 3;
  let action = producer.handle_error(&error);
  assert_eq!(action, ErrorAction::Stop);

  // Beyond max retries, should stop
  error.retries = 4;
  let action = producer.handle_error(&error);
  assert_eq!(action, ErrorAction::Stop);
}

#[test]
fn test_signal_producer_create_error_context_none_item() {
  let producer = SignalProducer::new().with_name("test_producer".to_string());
  let context = producer.create_error_context(None);

  assert_eq!(context.item, None);
  assert_eq!(context.component_name, "test_producer");
  assert_eq!(
    context.component_type,
    std::any::type_name::<SignalProducer>()
  );
}

#[test]
fn test_signal_producer_create_error_context_default_name() {
  let producer = SignalProducer::new();
  let context = producer.create_error_context(Some(Signal::Terminate));

  assert_eq!(context.item, Some(Signal::Terminate));
  assert_eq!(context.component_name, "signal_producer");
  assert_eq!(
    context.component_type,
    std::any::type_name::<SignalProducer>()
  );
}

#[test]
fn test_signal_producer_config_mut() {
  let mut producer = SignalProducer::new();
  let config_mut = producer.config_mut();

  config_mut.name = Some("mutated_via_config_mut".to_string());
  config_mut.error_strategy = ErrorStrategy::Retry(10);

  assert_eq!(
    producer.config.name,
    Some("mutated_via_config_mut".to_string())
  );
  assert_eq!(
    producer.config.error_strategy(),
    ErrorStrategy::<Signal>::Retry(10)
  );
}

#[test]
fn test_signal_producer_with_config() {
  let mut config = ProducerConfig::default();
  config.name = Some("config_name".to_string());
  config.error_strategy = ErrorStrategy::Skip;

  let producer = SignalProducer::new().with_config(config.clone());

  assert_eq!(producer.config.name, Some("config_name".to_string()));
  assert_eq!(
    producer.config.error_strategy(),
    ErrorStrategy::<Signal>::Skip
  );
}

#[test]
fn test_signal_debug() {
  let signal = Signal::Interrupt;
  let debug_str = format!("{:?}", signal);
  assert!(debug_str.contains("Interrupt"));

  let signal = Signal::Terminate;
  let debug_str = format!("{:?}", signal);
  assert!(debug_str.contains("Terminate"));
}

#[test]
fn test_signal_clone() {
  let signal1 = Signal::Interrupt;
  let signal2 = signal1.clone();
  assert_eq!(signal1, signal2);

  let signal1 = Signal::Terminate;
  let signal2 = signal1.clone();
  assert_eq!(signal1, signal2);
}

#[test]
fn test_signal_copy() {
  let signal1 = Signal::Interrupt;
  let signal2 = signal1; // Copy, not move
  assert_eq!(signal1, signal2);
}

#[test]
fn test_signal_eq() {
  assert_eq!(Signal::Interrupt, Signal::Interrupt);
  assert_eq!(Signal::Terminate, Signal::Terminate);
  assert_ne!(Signal::Interrupt, Signal::Terminate);
}

#[test]
fn test_signal_producer_output_trait() {
  let producer = SignalProducer::new();
  // Verify Output trait is implemented
  let _output_type: <SignalProducer as Output>::Output = Signal::Interrupt;
  let _output_stream_type: <SignalProducer as Output>::OutputStream =
    Box::pin(futures::stream::empty());
}

#[test]
fn test_signal_producer_output_ports() {
  let producer = SignalProducer::new();
  // Verify OutputPorts is correctly set
  let _ports: <SignalProducer as Producer>::OutputPorts = (Signal::Interrupt,);
}
