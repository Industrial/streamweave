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
}
