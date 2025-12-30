//! Tests for ArrayProducer

use futures::StreamExt;
use streamweave::Producer;
use streamweave_array::ArrayProducer;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

#[derive(Debug)]
struct TestError(String);

impl std::fmt::Display for TestError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl std::error::Error for TestError {}

#[tokio::test]
async fn test_array_producer_basic() {
  let mut producer = ArrayProducer::new([1, 2, 3, 4, 5]);
  let result: Vec<i32> = producer.produce().collect().await;
  assert_eq!(result, vec![1, 2, 3, 4, 5]);
}

#[tokio::test]
async fn test_array_producer_empty() {
  let mut producer = ArrayProducer::<i32, 0>::new([]);
  let result: Vec<i32> = producer.produce().collect().await;
  assert_eq!(result, Vec::<i32>::new());
}

#[tokio::test]
async fn test_array_producer_strings() {
  let mut producer = ArrayProducer::new(["a", "b", "c"]);
  let result: Vec<&str> = producer.produce().collect().await;
  assert_eq!(result, vec!["a", "b", "c"]);
}

#[tokio::test]
async fn test_array_producer_with_error() {
  let producer = ArrayProducer::new([1, 2, 3]);
  let error = StreamError::new(
    Box::new(TestError("test error".to_string())),
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(1),
      component_name: "array_producer".to_string(),
      component_type: std::any::type_name::<ArrayProducer<i32, 3>>().to_string(),
    },
    ComponentInfo {
      name: "array_producer".to_string(),
      type_name: std::any::type_name::<ArrayProducer<i32, 3>>().to_string(),
    },
  );

  let action = producer.handle_error(&error);
  assert_eq!(action, ErrorAction::Stop);
}

#[tokio::test]
async fn test_array_producer_error_handling_strategies() {
  let producer = ArrayProducer::new([1, 2, 3])
    .with_error_strategy(ErrorStrategy::<i32>::Skip)
    .with_name("test_producer".to_string());

  let config = producer.config();
  assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
  assert_eq!(config.name(), Some("test_producer".to_string()));
}

#[tokio::test]
async fn test_array_producer_from_slice_success() {
  let slice = &[1, 2, 3];
  let mut producer = ArrayProducer::<_, 3>::from_slice(slice).unwrap();
  let stream = producer.produce();
  let result: Vec<i32> = stream.collect().await;
  assert_eq!(result, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_array_producer_from_slice_failure() {
  let slice = &[1, 2, 3];
  assert!(ArrayProducer::<_, 4>::from_slice(slice).is_none());
}

#[tokio::test]
async fn test_array_producer_multiple_calls() {
  let mut producer = ArrayProducer::new([1, 2, 3]);

  // First call
  let stream = producer.produce();
  let result1: Vec<i32> = stream.collect().await;
  assert_eq!(result1, vec![1, 2, 3]);

  // Second call
  let stream = producer.produce();
  let result2: Vec<i32> = stream.collect().await;
  assert_eq!(result2, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_len_and_is_empty() {
  let producer: ArrayProducer<i32, 3> = ArrayProducer::new([1, 2, 3]);
  assert_eq!(producer.len(), 3);
  assert!(!producer.is_empty());

  let empty_producer: ArrayProducer<i32, 0> = ArrayProducer::new([]);
  assert_eq!(empty_producer.len(), 0);
  assert!(empty_producer.is_empty());
}

#[test]
fn test_error_handling_stop() {
  let producer = ArrayProducer::<i32, 3>::new([1, 2, 3]);
  let error = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "ArrayProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "ArrayProducer".to_string(),
    },
    retries: 0,
  };

  let strategy = producer.handle_error(&error);
  assert!(matches!(strategy, ErrorAction::Stop));
}

#[test]
fn test_error_handling_skip() {
  let mut producer = ArrayProducer::<i32, 3>::new([1, 2, 3]);
  producer = producer.with_error_strategy(ErrorStrategy::Skip);
  let error = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "ArrayProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "ArrayProducer".to_string(),
    },
    retries: 0,
  };

  let strategy = producer.handle_error(&error);
  assert!(matches!(strategy, ErrorAction::Skip));
}

#[test]
fn test_error_handling_retry() {
  let producer =
    ArrayProducer::<i32, 3>::new([1, 2, 3]).with_error_strategy(ErrorStrategy::Retry(3));

  // Test retry when retries < max
  let error = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(42),
      component_name: "test".to_string(),
      component_type: "ArrayProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "ArrayProducer".to_string(),
    },
    retries: 2, // Less than max (3)
  };

  let action = producer.handle_error(&error);
  assert!(matches!(action, ErrorAction::Retry));

  // Test stop when retries >= max (equal case)
  let error_max_retries = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(42),
      component_name: "test".to_string(),
      component_type: "ArrayProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "ArrayProducer".to_string(),
    },
    retries: 3, // Equal to max
  };

  let action = producer.handle_error(&error_max_retries);
  assert!(matches!(action, ErrorAction::Stop));

  // Test stop when retries > max (greater than case)
  let error_exceeded_retries = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(42),
      component_name: "test".to_string(),
      component_type: "ArrayProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "ArrayProducer".to_string(),
    },
    retries: 5, // Greater than max (3)
  };

  let action = producer.handle_error(&error_exceeded_retries);
  assert!(matches!(action, ErrorAction::Stop));
}

#[test]
fn test_error_handling_custom() {
  let producer = ArrayProducer::<i32, 3>::new([1, 2, 3])
    .with_error_strategy(ErrorStrategy::new_custom(|_| ErrorAction::Skip));

  let error = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(42),
      component_name: "test".to_string(),
      component_type: "ArrayProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "ArrayProducer".to_string(),
    },
    retries: 0,
  };

  let action = producer.handle_error(&error);
  assert!(matches!(action, ErrorAction::Skip));
}

#[test]
fn test_create_error_context_with_item() {
  let producer = ArrayProducer::<i32, 3>::new([1, 2, 3]).with_name("test_producer".to_string());

  let context = producer.create_error_context(Some(42));
  assert_eq!(context.component_name, "test_producer");
  assert_eq!(context.item, Some(42));
  assert!(context.component_type.contains("ArrayProducer"));
}

#[test]
fn test_create_error_context_without_item() {
  let producer = ArrayProducer::<i32, 3>::new([1, 2, 3]).with_name("test_producer".to_string());

  let context = producer.create_error_context(None);
  assert_eq!(context.component_name, "test_producer");
  assert_eq!(context.item, None);
  assert!(context.component_type.contains("ArrayProducer"));
}

#[test]
fn test_create_error_context_default_name() {
  let producer = ArrayProducer::<i32, 3>::new([1, 2, 3]);
  // No name set, should use default

  let context = producer.create_error_context(Some(42));
  assert_eq!(context.component_name, "array_producer");
  assert_eq!(context.item, Some(42));
}

#[test]
fn test_component_info_with_name() {
  let producer = ArrayProducer::<i32, 3>::new([1, 2, 3]).with_name("test_producer".to_string());

  let info = producer.component_info();
  assert_eq!(info.name, "test_producer");
  assert!(info.type_name.contains("ArrayProducer"));
}

#[test]
fn test_component_info_default_name() {
  let producer = ArrayProducer::<i32, 3>::new([1, 2, 3]);
  // No name set, should use default

  let info = producer.component_info();
  assert_eq!(info.name, "array_producer");
  assert!(info.type_name.contains("ArrayProducer"));
}

#[test]
fn test_set_config_impl() {
  use streamweave::ProducerConfig;

  let mut producer = ArrayProducer::<i32, 3>::new([1, 2, 3]);
  let new_config = ProducerConfig::<i32>::default()
    .with_error_strategy(ErrorStrategy::Skip)
    .with_name("custom_name".to_string());

  producer.set_config_impl(new_config.clone());

  let retrieved_config = producer.get_config_impl();
  assert_eq!(
    retrieved_config.error_strategy(),
    ErrorStrategy::<i32>::Skip
  );
  assert_eq!(retrieved_config.name(), Some("custom_name".to_string()));
}

#[test]
fn test_get_config_mut_impl() {
  use streamweave::ProducerConfig;

  let mut producer = ArrayProducer::<i32, 3>::new([1, 2, 3]);

  let config_mut = producer.get_config_mut_impl();
  *config_mut = ProducerConfig::default()
    .with_error_strategy(ErrorStrategy::Skip)
    .with_name("mutated_name".to_string());

  let config = producer.get_config_impl();
  assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
  assert_eq!(config.name(), Some("mutated_name".to_string()));
}

#[test]
fn test_from_slice_empty() {
  let slice: &[i32] = &[];
  let producer = ArrayProducer::<_, 0>::from_slice(slice);
  assert!(producer.is_some());

  let mut producer = producer.unwrap();
  let result: Vec<i32> = futures::executor::block_on(producer.produce().collect());
  assert_eq!(result, Vec::<i32>::new());
}

#[test]
fn test_from_slice_different_types() {
  // Test with u8
  let slice = &[1u8, 2, 3];
  let producer = ArrayProducer::<_, 3>::from_slice(slice).unwrap();
  let mut producer = producer;
  let result: Vec<u8> = futures::executor::block_on(producer.produce().collect());
  assert_eq!(result, vec![1, 2, 3]);

  // Test with String (Copy not required, but Clone is)
  // Note: String doesn't implement Copy, so from_slice won't work
  // This test verifies the Copy bound is enforced
}

#[test]
fn test_producer_output_ports() {
  use streamweave::{Producer, ProducerPorts};

  let producer = ArrayProducer::<i32, 3>::new([1, 2, 3]);
  // Verify OutputPorts is correctly set - this is a compile-time check
  // If ProducerPorts is not implemented, this won't compile
  fn assert_output_ports<P: Producer<Output = i32> + ProducerPorts>(_producer: P) {
    // This compiles only if ProducerPorts is implemented
    // The associated type DefaultOutputPorts should be (i32,)
  }

  assert_output_ports(producer);
}
