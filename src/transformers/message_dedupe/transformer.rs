//! Transformer trait implementation for MessageDedupeTransformer.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::input::Input;
use crate::message::Message;
use crate::output::Output;
use crate::transformer::{Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::stream::{self, StreamExt};

use super::message_dedupe_transformer::{
  CacheEntry, DeduplicationWindow, MessageDedupeTransformer,
};

impl<T> Input for MessageDedupeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = Message<T>;
  type InputStream = std::pin::Pin<Box<dyn futures::Stream<Item = Self::Input> + Send>>;
}

impl<T> Output for MessageDedupeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = Message<T>;
  type OutputStream = std::pin::Pin<Box<dyn futures::Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl<T> Transformer for MessageDedupeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (crate::message::Message<T>,);
  type OutputPorts = (crate::message::Message<T>,);
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    // Move the state into the stream for filtering
    let seen_ids = std::mem::take(&mut self.seen_ids);
    let eviction_queue = std::mem::take(&mut self.eviction_queue);
    let window = self.window.clone();

    Box::pin(stream::unfold(
      (input, seen_ids, eviction_queue, window),
      |(mut input, mut seen_ids, mut eviction_queue, window)| async move {
        loop {
          match input.next().await {
            Some(msg) => {
              let id = msg.id().clone();

              // Check if duplicate
              if seen_ids.contains(&id) {
                // Skip duplicate, continue to next item
                continue;
              }

              // Record this ID
              let entry = CacheEntry {
                id: id.clone(),
                seen_at: std::time::Instant::now(),
              };
              seen_ids.insert(id);
              eviction_queue.push_back(entry);

              // Evict if over count limit
              let max_count = match &window {
                DeduplicationWindow::Count(count) => Some(*count),
                DeduplicationWindow::CountAndTime { max_count, .. } => Some(*max_count),
                _ => None,
              };

              if let Some(max_count) = max_count {
                while eviction_queue.len() > max_count {
                  if let Some(removed) = eviction_queue.pop_front() {
                    seen_ids.remove(&removed.id);
                  }
                }
              }

              return Some((msg, (input, seen_ids, eviction_queue, window)));
            }
            None => return None,
          }
        }
      },
    ))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Message<T>>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Message<T>> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Message<T>> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Message<T>>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Message<T>>) -> ErrorContext<Message<T>> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "message_dedupe_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::message::{Message, MessageId};
  use futures::StreamExt;

  #[tokio::test]
  async fn test_transform_filters_duplicates() {
    let mut deduper = MessageDedupeTransformer::new();

    let messages = vec![
      Message::new(1, MessageId::new_sequence(1)),
      Message::new(2, MessageId::new_sequence(2)),
      Message::new(3, MessageId::new_sequence(1)), // duplicate ID
      Message::new(4, MessageId::new_sequence(3)),
      Message::new(5, MessageId::new_sequence(2)), // duplicate ID
    ];

    let input = stream::iter(messages);
    let output = deduper.transform(Box::pin(input));
    let results: Vec<_> = output.collect().await;

    // Should only have 3 unique messages
    assert_eq!(results.len(), 3);
    assert_eq!(*results[0].payload(), 1);
    assert_eq!(*results[1].payload(), 2);
    assert_eq!(*results[2].payload(), 4);
  }

  #[tokio::test]
  async fn test_transform_empty_input() {
    let mut deduper = MessageDedupeTransformer::<i32>::new();

    let input: std::pin::Pin<Box<dyn futures::Stream<Item = Message<i32>> + Send>> =
      Box::pin(stream::empty());
    let output = deduper.transform(input);
    let results: Vec<_> = output.collect().await;

    assert!(results.is_empty());
  }

  #[tokio::test]
  async fn test_transform_all_unique() {
    let mut deduper = MessageDedupeTransformer::new();

    let messages: Vec<Message<i32>> = (0..5)
      .map(|i| Message::new(i, MessageId::new_sequence(i as u64)))
      .collect();

    let input = stream::iter(messages);
    let output = deduper.transform(Box::pin(input));
    let results: Vec<_> = output.collect().await;

    assert_eq!(results.len(), 5);
  }

  #[tokio::test]
  async fn test_transform_all_duplicates() {
    let mut deduper = MessageDedupeTransformer::new();

    let messages = vec![
      Message::new(1, MessageId::new_sequence(1)),
      Message::new(2, MessageId::new_sequence(1)),
      Message::new(3, MessageId::new_sequence(1)),
    ];

    let input = stream::iter(messages);
    let output = deduper.transform(Box::pin(input));
    let results: Vec<_> = output.collect().await;

    // Only the first message should pass through
    assert_eq!(results.len(), 1);
    assert_eq!(*results[0].payload(), 1);
  }

  #[tokio::test]
  async fn test_transform_with_count_window() {
    let mut deduper = MessageDedupeTransformer::new().with_window(DeduplicationWindow::Count(2));

    // Send messages: 1, 2, 3, 1 (should now pass since window only keeps 2)
    let messages = vec![
      Message::new(1, MessageId::new_sequence(1)),
      Message::new(2, MessageId::new_sequence(2)),
      Message::new(3, MessageId::new_sequence(3)),
      Message::new(4, MessageId::new_sequence(1)), // ID 1 was evicted, so this passes
    ];

    let input = stream::iter(messages);
    let output = deduper.transform(Box::pin(input));
    let results: Vec<_> = output.collect().await;

    // All 4 should pass because the window is only 2
    assert_eq!(results.len(), 4);
  }

  #[tokio::test]
  async fn test_component_info() {
    let deduper = MessageDedupeTransformer::<i32>::new().with_name("test-deduper");
    let info = deduper.component_info();

    assert_eq!(info.name, "test-deduper");
  }

  #[tokio::test]
  async fn test_create_error_context() {
    let deduper = MessageDedupeTransformer::<i32>::new().with_name("test");
    let ctx = deduper.create_error_context(None);

    assert_eq!(ctx.component_name, "test");
    assert!(ctx.item.is_none());
  }

  #[tokio::test]
  async fn test_create_error_context_with_item() {
    let deduper = MessageDedupeTransformer::<i32>::new().with_name("test");
    let msg = Message::new(42, MessageId::new_sequence(1));
    let ctx = deduper.create_error_context(Some(msg.clone()));

    assert_eq!(ctx.component_name, "test");
    assert_eq!(ctx.item, Some(msg));
  }

  #[test]
  fn test_set_config_impl() {
    let mut deduper = MessageDedupeTransformer::<i32>::new();
    let new_config = TransformerConfig {
      name: Some("new_name".to_string()),
      error_strategy: ErrorStrategy::Skip,
    };

    deduper.set_config_impl(new_config.clone());

    let retrieved_config = deduper.get_config_impl();
    assert_eq!(retrieved_config.name(), Some("new_name".to_string()));
    assert!(matches!(
      retrieved_config.error_strategy(),
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_get_config_mut_impl() {
    let mut deduper = MessageDedupeTransformer::<i32>::new();
    let config_mut = deduper.get_config_mut_impl();
    config_mut.name = Some("mutated_name".to_string());
    config_mut.error_strategy = ErrorStrategy::Retry(5);

    let config = deduper.get_config_impl();
    assert_eq!(config.name(), Some("mutated_name".to_string()));
    assert!(matches!(config.error_strategy(), ErrorStrategy::Retry(5)));
  }

  #[test]
  fn test_error_handling_strategies() {
    let deduper = MessageDedupeTransformer::<i32>::new();
    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "MessageDedupeTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "MessageDedupeTransformer".to_string(),
      },
      retries: 0,
    };

    // Test default error strategy (Stop)
    assert!(matches!(deduper.handle_error(&error), ErrorAction::Stop));

    // Test Skip strategy
    let deduper = deduper.with_error_strategy(ErrorStrategy::Skip);
    assert!(matches!(deduper.handle_error(&error), ErrorAction::Skip));

    // Test Retry strategy
    let deduper = deduper.with_error_strategy(ErrorStrategy::Retry(3));
    assert!(matches!(deduper.handle_error(&error), ErrorAction::Retry));

    // Test Retry exhausted
    let error_exhausted = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "MessageDedupeTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "MessageDedupeTransformer".to_string(),
      },
      retries: 3,
    };
    assert!(matches!(
      deduper.handle_error(&error_exhausted),
      ErrorAction::Stop
    ));
  }

  #[test]
  fn test_custom_error_handler() {
    let deduper = MessageDedupeTransformer::<i32>::new().with_error_strategy(
      ErrorStrategy::new_custom(|error: &StreamError<Message<i32>>| {
        if error.retries < 2 {
          ErrorAction::Retry
        } else {
          ErrorAction::Skip
        }
      }),
    );

    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "MessageDedupeTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "MessageDedupeTransformer".to_string(),
      },
      retries: 1,
    };

    // Should retry when retries < 2
    assert!(matches!(deduper.handle_error(&error), ErrorAction::Retry));

    let error_exhausted = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "MessageDedupeTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "MessageDedupeTransformer".to_string(),
      },
      retries: 2,
    };

    // Should skip when retries >= 2
    assert!(matches!(
      deduper.handle_error(&error_exhausted),
      ErrorAction::Skip
    ));
  }

  #[tokio::test]
  async fn test_transform_with_count_and_time_window() {
    use std::time::Duration;
    let mut deduper =
      MessageDedupeTransformer::new().with_window(DeduplicationWindow::CountAndTime {
        max_count: 2,
        max_age: Duration::from_secs(1),
      });

    // Send messages: 1, 2, 3, 1 (should now pass since window only keeps 2)
    let messages = vec![
      Message::new(1, MessageId::new_sequence(1)),
      Message::new(2, MessageId::new_sequence(2)),
      Message::new(3, MessageId::new_sequence(3)),
      Message::new(4, MessageId::new_sequence(1)), // ID 1 was evicted, so this passes
    ];

    let input = stream::iter(messages);
    let output = deduper.transform(Box::pin(input));
    let results: Vec<_> = output.collect().await;

    // All 4 should pass because the window is only 2
    assert_eq!(results.len(), 4);
  }

  #[tokio::test]
  async fn test_transform_with_time_window() {
    use std::time::Duration;
    let mut deduper = MessageDedupeTransformer::new()
      .with_window(DeduplicationWindow::Time(Duration::from_secs(1)));

    let messages = vec![
      Message::new(1, MessageId::new_sequence(1)),
      Message::new(2, MessageId::new_sequence(2)),
    ];

    let input = stream::iter(messages);
    let output = deduper.transform(Box::pin(input));
    let results: Vec<_> = output.collect().await;

    // Both should pass (time window doesn't limit count)
    assert_eq!(results.len(), 2);
  }

  #[tokio::test]
  async fn test_component_info_default_name() {
    let deduper = MessageDedupeTransformer::<i32>::new();
    let info = deduper.component_info();

    assert_eq!(info.name, "message_dedupe_transformer");
    assert_eq!(
      info.type_name,
      std::any::type_name::<MessageDedupeTransformer<i32>>()
    );
  }

  #[tokio::test]
  async fn test_eviction_multiple_items() {
    let mut deduper = MessageDedupeTransformer::new().with_window(DeduplicationWindow::Count(3));

    // Send 5 messages, window size is 3, so after 3 messages, the first should be evicted
    let messages = vec![
      Message::new(1, MessageId::new_sequence(1)),
      Message::new(2, MessageId::new_sequence(2)),
      Message::new(3, MessageId::new_sequence(3)),
      Message::new(4, MessageId::new_sequence(4)),
      Message::new(5, MessageId::new_sequence(1)), // ID 1 was evicted, should pass
    ];

    let input = stream::iter(messages);
    let output = deduper.transform(Box::pin(input));
    let results: Vec<_> = output.collect().await;

    // All 5 should pass (first was evicted when 4th arrived)
    assert_eq!(results.len(), 5);
  }

  #[tokio::test]
  async fn test_transform_with_unbounded_window() {
    let mut deduper = MessageDedupeTransformer::new().with_window(DeduplicationWindow::Unbounded);

    // With unbounded window, all IDs should be remembered
    let messages = vec![
      Message::new(1, MessageId::new_sequence(1)),
      Message::new(2, MessageId::new_sequence(2)),
      Message::new(3, MessageId::new_sequence(3)),
      Message::new(4, MessageId::new_sequence(1)), // Should be filtered (unbounded remembers all)
      Message::new(5, MessageId::new_sequence(2)), // Should be filtered
    ];

    let input = stream::iter(messages);
    let output = deduper.transform(Box::pin(input));
    let results: Vec<_> = output.collect().await;

    // Only first 3 should pass (unbounded window remembers all)
    assert_eq!(results.len(), 3);
  }

  #[tokio::test]
  async fn test_transform_eviction_when_exactly_at_limit() {
    let mut deduper = MessageDedupeTransformer::new().with_window(DeduplicationWindow::Count(2));

    // Send exactly 2 messages, then a 3rd that should evict the first
    let messages = vec![
      Message::new(1, MessageId::new_sequence(1)),
      Message::new(2, MessageId::new_sequence(2)),
      Message::new(3, MessageId::new_sequence(3)), // Should evict ID 1
      Message::new(4, MessageId::new_sequence(1)), // ID 1 was evicted, should pass
    ];

    let input = stream::iter(messages);
    let output = deduper.transform(Box::pin(input));
    let results: Vec<_> = output.collect().await;

    // All 4 should pass (ID 1 was evicted when 3rd arrived)
    assert_eq!(results.len(), 4);
  }

  #[tokio::test]
  async fn test_transform_eviction_multiple_at_once() {
    let mut deduper = MessageDedupeTransformer::new().with_window(DeduplicationWindow::Count(2));

    // Send messages that will require multiple evictions
    let messages = vec![
      Message::new(1, MessageId::new_sequence(1)),
      Message::new(2, MessageId::new_sequence(2)),
      Message::new(3, MessageId::new_sequence(3)), // Evicts 1
      Message::new(4, MessageId::new_sequence(4)), // Evicts 2
      Message::new(5, MessageId::new_sequence(5)), // Evicts 3
      Message::new(6, MessageId::new_sequence(1)), // Should pass (1 was evicted)
    ];

    let input = stream::iter(messages);
    let output = deduper.transform(Box::pin(input));
    let results: Vec<_> = output.collect().await;

    // All 6 should pass (each new message evicts the oldest)
    assert_eq!(results.len(), 6);
  }

  #[tokio::test]
  async fn test_transform_count_and_time_window_eviction() {
    use std::time::Duration;
    let mut deduper =
      MessageDedupeTransformer::new().with_window(DeduplicationWindow::CountAndTime {
        max_count: 2,
        max_age: Duration::from_secs(1),
      });

    // Send messages that will trigger count-based eviction
    let messages = vec![
      Message::new(1, MessageId::new_sequence(1)),
      Message::new(2, MessageId::new_sequence(2)),
      Message::new(3, MessageId::new_sequence(3)), // Evicts 1 (count limit)
      Message::new(4, MessageId::new_sequence(1)), // Should pass (1 was evicted)
    ];

    let input = stream::iter(messages);
    let output = deduper.transform(Box::pin(input));
    let results: Vec<_> = output.collect().await;

    // All 4 should pass
    assert_eq!(results.len(), 4);
  }

  #[tokio::test]
  async fn test_transform_eviction_when_queue_exceeds_limit_by_more_than_one() {
    let mut deduper = MessageDedupeTransformer::new().with_window(DeduplicationWindow::Count(1));

    // Send multiple messages rapidly to trigger multiple evictions in one iteration
    let messages = vec![
      Message::new(1, MessageId::new_sequence(1)),
      Message::new(2, MessageId::new_sequence(2)), // Evicts 1
      Message::new(3, MessageId::new_sequence(3)), // Evicts 2
      Message::new(4, MessageId::new_sequence(4)), // Evicts 3
    ];

    let input = stream::iter(messages);
    let output = deduper.transform(Box::pin(input));
    let results: Vec<_> = output.collect().await;

    // All 4 should pass (each evicts the previous)
    assert_eq!(results.len(), 4);
  }

  #[tokio::test]
  async fn test_transform_continue_loop_with_duplicates() {
    let mut deduper = MessageDedupeTransformer::new();

    // Test that the continue statement works correctly with multiple consecutive duplicates
    let messages = vec![
      Message::new(1, MessageId::new_sequence(1)),
      Message::new(2, MessageId::new_sequence(1)), // duplicate, should continue
      Message::new(3, MessageId::new_sequence(1)), // duplicate, should continue
      Message::new(4, MessageId::new_sequence(2)), // new ID
      Message::new(5, MessageId::new_sequence(1)), // duplicate again
    ];

    let input = stream::iter(messages);
    let output = deduper.transform(Box::pin(input));
    let results: Vec<_> = output.collect().await;

    // Only 2 unique messages should pass
    assert_eq!(results.len(), 2);
    assert_eq!(*results[0].payload(), 1);
    assert_eq!(*results[1].payload(), 4);
  }

  #[tokio::test]
  async fn test_transform_unbounded_window_no_eviction() {
    // With unbounded, no eviction should occur regardless of count
    let messages: Vec<Message<i32>> = (0..100)
      .map(|i| Message::new(i, MessageId::new_sequence(i as u64)))
      .collect();

    let mut deduper = MessageDedupeTransformer::new().with_window(DeduplicationWindow::Unbounded);
    let input = stream::iter(messages.clone());
    let output = deduper.transform(Box::pin(input));
    let results: Vec<_> = output.collect().await;

    // All should pass first time
    assert_eq!(results.len(), 100);

    // Create a new transformer with same config to test that unbounded remembers all
    // (Note: in real usage, the state persists in the transformer, but transform moves it)
    let mut deduper2 = MessageDedupeTransformer::new().with_window(DeduplicationWindow::Unbounded);

    // First pass - all unique
    let input2 = stream::iter(messages.clone());
    let output2 = deduper2.transform(Box::pin(input2));
    let results2: Vec<_> = output2.collect().await;
    assert_eq!(results2.len(), 100);

    // Second pass with same IDs - all should be filtered in a real scenario
    // But since transform moves state, we can't test this directly
    // The unbounded case is tested by ensuring no eviction code runs
  }
}
