//! Tests for VecProducer

use futures::StreamExt;
use streamweave::error::ErrorStrategy;
use streamweave::producers::VecProducer;
use streamweave::{Producer, ProducerConfig};

#[tokio::test]
async fn test_vec_producer_basic() {
  let mut producer = VecProducer::new(vec![1, 2, 3, 4, 5]);
  let mut stream = producer.produce();

  let mut results = Vec::new();
  while let Some(item) = stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3, 4, 5]);
}

#[tokio::test]
async fn test_vec_producer_empty() {
  let mut producer = VecProducer::new(vec![] as Vec<i32>);
  let mut stream = producer.produce();

  assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn test_vec_producer_single_item() {
  let mut producer = VecProducer::new(vec![42]);
  let mut stream = producer.produce();

  assert_eq!(stream.next().await, Some(42));
  assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn test_vec_producer_with_name() {
  let producer = VecProducer::new(vec![1, 2, 3]).with_name("test_producer".to_string());

  let config = producer.get_config_impl();
  assert_eq!(config.name(), Some("test_producer"));
}

#[tokio::test]
async fn test_vec_producer_with_error_strategy() {
  let producer = VecProducer::new(vec![1, 2, 3]).with_error_strategy(ErrorStrategy::Skip);

  let config = producer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_vec_producer_config_operations() {
  let mut producer = VecProducer::new(vec![1, 2, 3]);

  let config = producer.get_config_mut_impl();
  config.name = Some("updated".to_string());

  assert_eq!(producer.get_config_impl().name(), Some("updated"));
}

#[tokio::test]
async fn test_vec_producer_set_config() {
  let mut producer = VecProducer::new(vec![1, 2, 3]);
  let mut config = ProducerConfig::default();
  config.name = Some("new_name".to_string());
  producer.set_config_impl(config);

  assert_eq!(producer.get_config_impl().name(), Some("new_name"));
}

#[tokio::test]
async fn test_vec_producer_component_info() {
  let producer = VecProducer::new(vec![1, 2, 3]).with_name("test".to_string());

  let info = producer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("VecProducer"));
}

#[tokio::test]
async fn test_vec_producer_clone() {
  let producer1 = VecProducer::new(vec![1, 2, 3]);
  let producer2 = producer1.clone();

  let mut stream1 = producer1.produce();
  let mut stream2 = producer2.produce();

  assert_eq!(stream1.next().await, Some(1));
  assert_eq!(stream2.next().await, Some(1));
}
