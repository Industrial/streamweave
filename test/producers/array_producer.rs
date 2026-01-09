//! Tests for ArrayProducer

use futures::StreamExt;
use streamweave::error::ErrorStrategy;
use streamweave::producers::ArrayProducer;
use streamweave::{Producer, ProducerConfig};

#[tokio::test]
async fn test_array_producer_basic() {
  let mut producer = ArrayProducer::new([1, 2, 3, 4, 5]);
  let mut stream = producer.produce();

  let mut results = Vec::new();
  while let Some(item) = stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3, 4, 5]);
}

#[tokio::test]
async fn test_array_producer_empty() {
  let mut producer: ArrayProducer<i32, 0> = ArrayProducer::new([]);
  let mut stream = producer.produce();

  assert!(stream.next().await.is_none());
  assert!(producer.is_empty());
  assert_eq!(producer.len(), 0);
}

#[tokio::test]
async fn test_array_producer_single_item() {
  let mut producer = ArrayProducer::new([42]);
  let mut stream = producer.produce();

  assert_eq!(stream.next().await, Some(42));
  assert!(stream.next().await.is_none());
  assert_eq!(producer.len(), 1);
  assert!(!producer.is_empty());
}

#[tokio::test]
async fn test_array_producer_from_slice() {
  let slice = &[10, 20, 30];
  let producer = ArrayProducer::<i32, 3>::from_slice(slice);

  assert!(producer.is_some());
  let mut producer = producer.unwrap();
  let mut stream = producer.produce();

  let mut results = Vec::new();
  while let Some(item) = stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![10, 20, 30]);
}

#[tokio::test]
async fn test_array_producer_from_slice_wrong_length() {
  let slice = &[10, 20, 30];
  let producer = ArrayProducer::<i32, 5>::from_slice(slice);

  assert!(producer.is_none());
}

#[tokio::test]
async fn test_array_producer_with_name() {
  let producer = ArrayProducer::new([1, 2, 3]).with_name("test_producer".to_string());

  let config = producer.get_config_impl();
  assert_eq!(config.name(), Some("test_producer"));
}

#[tokio::test]
async fn test_array_producer_with_error_strategy() {
  let producer = ArrayProducer::new([1, 2, 3]).with_error_strategy(ErrorStrategy::Skip);

  let config = producer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_array_producer_config_mut() {
  let mut producer = ArrayProducer::new([1, 2, 3]);
  let config = producer.get_config_mut_impl();
  config.name = Some("updated".to_string());

  assert_eq!(producer.get_config_impl().name(), Some("updated"));
}

#[tokio::test]
async fn test_array_producer_set_config() {
  let mut producer = ArrayProducer::new([1, 2, 3]);
  let mut config = ProducerConfig::default();
  config.name = Some("new_name".to_string());
  producer.set_config_impl(config);

  assert_eq!(producer.get_config_impl().name(), Some("new_name"));
}

#[tokio::test]
async fn test_array_producer_component_info() {
  let producer = ArrayProducer::new([1, 2, 3]).with_name("test".to_string());

  let info = producer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("ArrayProducer"));
}

#[tokio::test]
async fn test_array_producer_strings() {
  let mut producer = ArrayProducer::new(["hello", "world", "test"]);
  let mut stream = producer.produce();

  let mut results = Vec::new();
  while let Some(item) = stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec!["hello", "world", "test"]);
}
