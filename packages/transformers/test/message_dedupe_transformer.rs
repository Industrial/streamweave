use futures::{StreamExt, stream};
use std::thread;
use std::time::Duration;
use streamweave::TransformerConfig;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_message::{Message, MessageId};
use streamweave_transformers::{DeduplicationWindow, MessageDedupeTransformer};

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
  let output = deduper.transform(Box::pin(input)).await;
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
  let output = deduper.transform(input).await;
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
  let output = deduper.transform(Box::pin(input)).await;
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
  let output = deduper.transform(Box::pin(input)).await;
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
  let output = deduper.transform(Box::pin(input)).await;
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
fn test_new_default() {
  let deduper = MessageDedupeTransformer::<i32>::new();
  assert_eq!(deduper.cache_size(), 0);
  assert_eq!(deduper.messages_processed(), 0);
  assert_eq!(deduper.duplicates_filtered(), 0);
}

#[test]
fn test_record_unique_ids() {
  let mut deduper = MessageDedupeTransformer::<i32>::new();

  assert!(!deduper.record_id(MessageId::new_sequence(1)));
  assert!(!deduper.record_id(MessageId::new_sequence(2)));
  assert!(!deduper.record_id(MessageId::new_sequence(3)));

  assert_eq!(deduper.cache_size(), 3);
  assert_eq!(deduper.messages_processed(), 3);
  assert_eq!(deduper.duplicates_filtered(), 0);
}

#[test]
fn test_detect_duplicates() {
  let mut deduper = MessageDedupeTransformer::<i32>::new();

  assert!(!deduper.record_id(MessageId::new_sequence(1)));
  assert!(deduper.record_id(MessageId::new_sequence(1))); // duplicate
  assert!(!deduper.record_id(MessageId::new_sequence(2)));
  assert!(deduper.record_id(MessageId::new_sequence(1))); // duplicate again

  assert_eq!(deduper.cache_size(), 2);
  assert_eq!(deduper.messages_processed(), 4);
  assert_eq!(deduper.duplicates_filtered(), 2);
}

#[test]
fn test_count_window_eviction() {
  let mut deduper =
    MessageDedupeTransformer::<i32>::new().with_window(DeduplicationWindow::Count(3));

  deduper.record_id(MessageId::new_sequence(1));
  deduper.record_id(MessageId::new_sequence(2));
  deduper.record_id(MessageId::new_sequence(3));
  assert_eq!(deduper.cache_size(), 3);

  // Adding a 4th should evict the 1st
  deduper.record_id(MessageId::new_sequence(4));
  assert_eq!(deduper.cache_size(), 3);

  // ID 1 should no longer be detected as duplicate
  assert!(!deduper.record_id(MessageId::new_sequence(1)));
}

#[test]
fn test_time_window_eviction() {
  let mut deduper = MessageDedupeTransformer::<i32>::new()
    .with_window(DeduplicationWindow::Time(Duration::from_millis(100)));

  deduper.record_id(MessageId::new_sequence(1));
  assert_eq!(deduper.cache_size(), 1);

  // Should still be there immediately
  assert!(deduper.is_duplicate(&MessageId::new_sequence(1)));

  // Wait for expiration
  thread::sleep(Duration::from_millis(150));

  // Recording a new ID should trigger eviction
  deduper.record_id(MessageId::new_sequence(2));

  // ID 1 should be expired
  assert!(!deduper.is_duplicate(&MessageId::new_sequence(1)));
}

#[test]
fn test_deduplication_ratio() {
  let mut deduper = MessageDedupeTransformer::<i32>::new();

  // 0 messages = 0.0 ratio
  assert_eq!(deduper.deduplication_ratio(), 0.0);

  // 4 messages, 2 duplicates = 0.5 ratio
  deduper.record_id(MessageId::new_sequence(1));
  deduper.record_id(MessageId::new_sequence(1)); // dup
  deduper.record_id(MessageId::new_sequence(2));
  deduper.record_id(MessageId::new_sequence(2)); // dup

  assert_eq!(deduper.deduplication_ratio(), 0.5);
}

#[test]
fn test_clear() {
  let mut deduper = MessageDedupeTransformer::<i32>::new();

  deduper.record_id(MessageId::new_sequence(1));
  deduper.record_id(MessageId::new_sequence(2));
  assert_eq!(deduper.cache_size(), 2);

  deduper.clear();
  assert_eq!(deduper.cache_size(), 0);
  assert_eq!(deduper.messages_processed(), 0);
  assert_eq!(deduper.duplicates_filtered(), 0);
}

#[test]
fn test_with_name() {
  let deduper = MessageDedupeTransformer::<i32>::new().with_name("my-deduper");
  assert_eq!(deduper.config.name, Some("my-deduper".to_string()));
}

#[test]
fn test_clone_has_empty_cache() {
  let mut deduper = MessageDedupeTransformer::<i32>::new();
  deduper.record_id(MessageId::new_sequence(1));
  deduper.record_id(MessageId::new_sequence(2));

  let cloned = deduper.clone();
  // Original has cached IDs
  assert_eq!(deduper.cache_size(), 2);
  // Clone starts fresh (for parallel processing)
  assert_eq!(cloned.cache_size(), 0);
}

#[test]
fn test_different_id_types() {
  let mut deduper = MessageDedupeTransformer::<String>::new();

  // UUID
  deduper.record_id(MessageId::new_uuid());
  deduper.record_id(MessageId::new_uuid());

  // Sequence
  deduper.record_id(MessageId::new_sequence(1));

  // Custom
  deduper.record_id(MessageId::new_custom("custom-id-1"));
  assert!(deduper.record_id(MessageId::new_custom("custom-id-1"))); // dup

  // Content hash
  deduper.record_id(MessageId::from_content(b"hello"));
  assert!(deduper.record_id(MessageId::from_content(b"hello"))); // dup

  assert_eq!(deduper.duplicates_filtered(), 2);
}

#[test]
fn test_with_error_strategy() {
  let deduper =
    MessageDedupeTransformer::<i32>::new().with_error_strategy(ErrorStrategy::<Message<i32>>::Skip);
  assert!(matches!(deduper.config.error_strategy, ErrorStrategy::Skip));
}

#[test]
fn test_window_getter() {
  let deduper = MessageDedupeTransformer::<i32>::new().with_window(DeduplicationWindow::Count(100));
  match deduper.window() {
    DeduplicationWindow::Count(100) => {}
    _ => panic!("Expected Count(100)"),
  }
}

#[test]
fn test_default_impl() {
  let deduper = MessageDedupeTransformer::<i32>::default();
  assert_eq!(deduper.cache_size(), 0);
  match deduper.window() {
    DeduplicationWindow::Count(10000) => {}
    _ => panic!("Expected default Count(10000)"),
  }
}
