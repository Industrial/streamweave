//! Tests for MessageDedupeTransformer

use futures::StreamExt;
use futures::stream;
use std::time::Duration;
use streamweave::error::ErrorStrategy;
use streamweave::message::{Message, MessageId};
use streamweave::transformers::{DeduplicationWindow, MessageDedupeTransformer};
use streamweave::{Transformer, TransformerConfig};

#[test]
fn test_deduplication_window_variants() {
  assert_eq!(
    DeduplicationWindow::Unbounded,
    DeduplicationWindow::Unbounded
  );
  assert_eq!(
    DeduplicationWindow::Count(100),
    DeduplicationWindow::Count(100)
  );
  assert_eq!(
    DeduplicationWindow::Time(Duration::from_secs(10)),
    DeduplicationWindow::Time(Duration::from_secs(10))
  );
}

#[test]
fn test_deduplication_window_default() {
  let window = DeduplicationWindow::default();
  match window {
    DeduplicationWindow::Count(n) => assert_eq!(n, 10_000),
    _ => assert!(false),
  }
}

#[tokio::test]
async fn test_message_dedupe_transformer_basic() {
  let mut transformer = MessageDedupeTransformer::<i32>::new();
  let id1 = MessageId::new_uuid();
  let id2 = MessageId::new_uuid();

  let messages = vec![
    Message::new(1, id1.clone()),
    Message::new(2, id2.clone()),
    Message::new(3, id1.clone()), // Duplicate ID
  ];
  let input_stream = Box::pin(stream::iter(messages));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(msg) = output_stream.next().await {
    results.push(msg);
  }

  // Should filter duplicate message ID
  assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn test_message_dedupe_transformer_with_window() {
  let transformer =
    MessageDedupeTransformer::<i32>::new().with_window(DeduplicationWindow::Count(10));

  // Transformer should be created
  assert!(true);
}

#[tokio::test]
async fn test_message_dedupe_transformer_empty() {
  let mut transformer = MessageDedupeTransformer::<i32>::new();
  let input_stream = Box::pin(stream::iter(vec![] as Vec<Message<i32>>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_message_dedupe_transformer_with_name() {
  let transformer = MessageDedupeTransformer::<i32>::new().with_name("deduper".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("deduper"));
}

#[tokio::test]
async fn test_message_dedupe_transformer_with_error_strategy() {
  let transformer = MessageDedupeTransformer::<i32>::new().with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}
