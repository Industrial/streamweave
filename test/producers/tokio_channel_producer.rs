//! Tests for TokioChannelProducer

use futures::StreamExt;
use streamweave::error::ErrorStrategy;
use streamweave::producers::TokioChannelProducer;
use streamweave::{Producer, ProducerConfig};
use tokio::sync::mpsc;

#[tokio::test]
async fn test_channel_producer_basic() {
  let (tx, rx) = mpsc::channel(10);
  let mut producer = TokioChannelProducer::new(rx);

  // Send some items
  tx.send(1).await.unwrap();
  tx.send(2).await.unwrap();
  tx.send(3).await.unwrap();
  drop(tx); // Close the channel

  let mut stream = producer.produce();
  let mut results = Vec::new();
  while let Some(item) = stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_channel_producer_empty() {
  let (tx, rx) = mpsc::channel(10);
  drop(tx); // Close immediately

  let mut producer = TokioChannelProducer::new(rx);
  let mut stream = producer.produce();

  assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn test_channel_producer_with_name() {
  let (_tx, rx) = mpsc::channel(10);
  let producer = ChannelProducer::new(rx).with_name("channel_producer".to_string());

  let config = producer.get_config_impl();
  assert_eq!(config.name(), Some("channel_producer"));
}

#[tokio::test]
async fn test_channel_producer_with_error_strategy() {
  let (_tx, rx) = mpsc::channel(10);
  let producer = ChannelProducer::new(rx).with_error_strategy(ErrorStrategy::Skip);

  let config = producer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_channel_producer_config_operations() {
  let (_tx, rx) = mpsc::channel(10);
  let mut producer = TokioChannelProducer::new(rx);

  let config = producer.get_config_mut_impl();
  config.name = Some("updated".to_string());

  assert_eq!(producer.get_config_impl().name(), Some("updated"));
}

#[tokio::test]
async fn test_channel_producer_component_info() {
  let (_tx, rx) = mpsc::channel(10);
  let producer = TokioChannelProducer::new(rx).with_name("test".to_string());

  let info = producer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("TokioChannelProducer"));
}
