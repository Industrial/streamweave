//! Tests for DeadLetterQueue

use futures::stream;
use streamweave::consumers::DeadLetterQueue;
use streamweave::error::ErrorStrategy;
use streamweave::{Consumer, ConsumerConfig};

#[tokio::test]
async fn test_dead_letter_queue_basic() {
  let mut dlq = DeadLetterQueue::new();
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  dlq.consume(input_stream).await;

  let queue = dlq.queue.lock().await;
  assert_eq!(queue.len(), 3);
  assert_eq!(queue[0].item, 1);
  assert_eq!(queue[1].item, 2);
  assert_eq!(queue[2].item, 3);
}

#[tokio::test]
async fn test_dead_letter_queue_empty() {
  let mut dlq = DeadLetterQueue::new();
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  dlq.consume(input_stream).await;

  let queue = dlq.queue.lock().await;
  assert_eq!(queue.len(), 0);
}

#[tokio::test]
async fn test_dead_letter_queue_with_capacity() {
  let dlq = DeadLetterQueue::<i32>::with_capacity(100);

  let queue = dlq.queue.lock().await;
  assert_eq!(queue.capacity(), 100);
}

#[tokio::test]
async fn test_dead_letter_queue_default() {
  let dlq = DeadLetterQueue::<i32>::default();

  let queue = dlq.queue.lock().await;
  assert_eq!(queue.len(), 0);
}

#[tokio::test]
async fn test_dead_letter_queue_with_name() {
  let mut dlq = DeadLetterQueue::new().with_name("dlq".to_string());

  assert_eq!(dlq.config.name, "dlq");
}

#[tokio::test]
async fn test_dead_letter_queue_with_error_strategy() {
  let dlq = DeadLetterQueue::new().with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(dlq.config.error_strategy, ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_dead_letter_queue_get_items() {
  let mut dlq = DeadLetterQueue::new();
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  dlq.consume(input_stream).await;

  let items = dlq.get_items().await;
  assert_eq!(items.len(), 3);
  assert_eq!(items[0].item, 1);
}

#[tokio::test]
async fn test_dead_letter_queue_clear() {
  let mut dlq = DeadLetterQueue::new();
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  dlq.consume(input_stream).await;
  dlq.clear().await;

  let queue = dlq.queue.lock().await;
  assert_eq!(queue.len(), 0);
}

#[tokio::test]
async fn test_dead_letter_queue_len() {
  let mut dlq = DeadLetterQueue::new();
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  dlq.consume(input_stream).await;
  assert_eq!(dlq.len().await, 3);
}

#[tokio::test]
async fn test_dead_letter_queue_is_empty() {
  let dlq = DeadLetterQueue::<i32>::new();
  assert!(dlq.is_empty().await);

  let mut dlq2 = DeadLetterQueue::new();
  let input_stream = Box::pin(stream::iter(vec![1]));
  dlq2.consume(input_stream).await;
  assert!(!dlq2.is_empty().await);
}

#[tokio::test]
async fn test_dead_letter_queue_component_info() {
  let dlq = DeadLetterQueue::new().with_name("test".to_string());

  let info = dlq.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("DeadLetterQueue"));
}
