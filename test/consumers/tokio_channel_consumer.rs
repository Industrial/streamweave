//! Tests for TokioChannelConsumer

use futures::stream;
use streamweave::consumers::TokioChannelConsumer;
use streamweave::error::ErrorStrategy;
use streamweave::{Consumer, ConsumerConfig};
use tokio::sync::mpsc;

#[tokio::test]
async fn test_channel_consumer_basic() {
  let (tx, mut rx) = mpsc::channel(10);
  let mut consumer = TokioChannelConsumer::new(tx);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

  consumer.consume(input_stream).await;
  drop(consumer); // Close the consumer to close the channel

  let mut results = Vec::new();
  while let Some(item) = rx.recv().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3, 4, 5]);
}

#[tokio::test]
async fn test_channel_consumer_empty() {
  let (tx, mut rx) = mpsc::channel(10);
  let mut consumer = TokioChannelConsumer::new(tx);
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  consumer.consume(input_stream).await;
  drop(consumer);

  assert!(rx.recv().await.is_none());
}

#[tokio::test]
async fn test_channel_consumer_with_name() {
  let (tx, _rx) = mpsc::channel(10);
  let consumer = ChannelConsumer::new(tx).with_name("channel_consumer".to_string());

  assert_eq!(consumer.config.name, "channel_consumer");
}

#[tokio::test]
async fn test_channel_consumer_with_error_strategy() {
  let (tx, _rx) = mpsc::channel(10);
  let consumer = ChannelConsumer::new(tx).with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(
    consumer.config.error_strategy,
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_channel_consumer_config_operations() {
  let (tx, _rx) = mpsc::channel(10);
  let mut consumer = TokioChannelConsumer::new(tx);

  let mut config = ConsumerConfig::default();
  config.name = "updated".to_string();
  consumer.set_config_impl(config);

  assert_eq!(consumer.get_config_impl().name, "updated");
}

#[tokio::test]
async fn test_channel_consumer_component_info() {
  let (tx, _rx) = mpsc::channel(10);
  let consumer = TokioChannelConsumer::new(tx).with_name("test".to_string());

  let info = consumer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("TokioChannelConsumer"));
}
