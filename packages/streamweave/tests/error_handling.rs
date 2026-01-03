//! Integration tests for error handling with Message<T>
//!
//! These tests verify:
//! - Error contexts include message information (IDs, metadata)
//! - Error strategies work correctly (Stop, Skip, Retry, Custom)
//! - Error recovery preserves message integrity

use futures::StreamExt;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::message::{Message, MessageId, MessageMetadata, wrap_message};
use streamweave::{Consumer, Input, Output, Producer, Transformer};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorStrategy, StreamError};
use tokio::sync::Mutex;
use tokio_stream::Stream;

// ============================================================================
// Test Error Type
// ============================================================================

#[derive(Debug)]
struct TestError(String);

impl std::fmt::Display for TestError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl std::error::Error for TestError {}

// ============================================================================
// Test Components
// ============================================================================

/// Producer that can fail on specific items
#[derive(Clone)]
struct FailingProducer {
  items: Vec<i32>,
  fail_on: Option<i32>,
  config: streamweave::ProducerConfig<Message<i32>>,
}

impl FailingProducer {
  fn new(items: Vec<i32>) -> Self {
    Self {
      items,
      fail_on: None,
      config: streamweave::ProducerConfig::default(),
    }
  }

  #[allow(dead_code)]
  fn fail_on(mut self, value: i32) -> Self {
    self.fail_on = Some(value);
    self
  }
}

impl Output for FailingProducer {
  type Output = Message<i32>;
  type OutputStream = Pin<Box<dyn Stream<Item = Message<i32>> + Send>>;
}

#[async_trait::async_trait]
impl Producer for FailingProducer {
  type OutputPorts = (Message<i32>,);

  fn produce(&mut self) -> Self::OutputStream {
    let items = self.items.clone();
    let fail_on = self.fail_on;
    Box::pin(futures::stream::iter(items.into_iter().map(move |item| {
      if fail_on == Some(item) {
        // In a real scenario, this would be handled by error handling infrastructure
        // For testing, we'll simulate this differently
      }
      wrap_message(item)
    })))
  }

  fn set_config_impl(&mut self, config: streamweave::ProducerConfig<Self::Output>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &streamweave::ProducerConfig<Self::Output> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut streamweave::ProducerConfig<Self::Output> {
    &mut self.config
  }
}

/// Transformer that fails on specific payload values
#[derive(Clone)]
struct FailingTransformer {
  fail_on: Option<i32>,
  config: streamweave::TransformerConfig<Message<i32>>,
}

impl FailingTransformer {
  fn new() -> Self {
    Self {
      fail_on: None,
      config: streamweave::TransformerConfig::default(),
    }
  }

  #[allow(dead_code)]
  fn fail_on(mut self, value: i32) -> Self {
    self.fail_on = Some(value);
    self
  }
}

impl Input for FailingTransformer {
  type Input = Message<i32>;
  type InputStream = Pin<Box<dyn Stream<Item = Message<i32>> + Send>>;
}

impl Output for FailingTransformer {
  type Output = Message<i32>;
  type OutputStream = Pin<Box<dyn Stream<Item = Message<i32>> + Send>>;
}

#[async_trait::async_trait]
impl Transformer for FailingTransformer {
  type InputPorts = (Message<i32>,);
  type OutputPorts = (Message<i32>,);

  async fn transform(&mut self, stream: Self::InputStream) -> Self::OutputStream {
    let fail_on = self.fail_on;
    Box::pin(stream.map(move |msg| {
      let payload = *msg.payload();
      if fail_on == Some(payload) {
        // In a real scenario, this would trigger error handling
        // For testing, we'll simulate this differently
      }
      let id = msg.id().clone();
      let metadata = msg.metadata().clone();
      Message::with_metadata(payload * 2, id, metadata)
    }))
  }

  fn set_config_impl(&mut self, config: streamweave::TransformerConfig<Self::Input>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &streamweave::TransformerConfig<Self::Input> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut streamweave::TransformerConfig<Self::Input> {
    &mut self.config
  }
}

/// Consumer that collects messages and can track errors
#[derive(Clone)]
struct CollectingConsumer {
  items: Arc<Mutex<Vec<Message<i32>>>>,
  #[allow(dead_code)]
  errors: Arc<Mutex<Vec<StreamError<Message<i32>>>>>,
  config: streamweave::ConsumerConfig<Message<i32>>,
}

impl CollectingConsumer {
  fn new() -> Self {
    Self {
      items: Arc::new(Mutex::new(Vec::new())),
      errors: Arc::new(Mutex::new(Vec::new())),
      config: streamweave::ConsumerConfig::default(),
    }
  }

  #[allow(dead_code, clippy::type_complexity)]
  fn into_inner(
    self,
  ) -> (
    Arc<Mutex<Vec<Message<i32>>>>,
    Arc<Mutex<Vec<StreamError<Message<i32>>>>>,
  ) {
    (self.items, self.errors)
  }
}

impl Input for CollectingConsumer {
  type Input = Message<i32>;
  type InputStream = Pin<Box<dyn Stream<Item = Message<i32>> + Send>>;
}

#[async_trait::async_trait]
impl Consumer for CollectingConsumer {
  type InputPorts = (Message<i32>,);

  async fn consume(&mut self, stream: Self::InputStream) {
    let items = self.items.clone();
    stream
      .for_each(|msg| {
        let items = items.clone();
        async move {
          items.lock().await.push(msg);
        }
      })
      .await;
  }

  fn set_config_impl(&mut self, config: streamweave::ConsumerConfig<Self::Input>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &streamweave::ConsumerConfig<Self::Input> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut streamweave::ConsumerConfig<Self::Input> {
    &mut self.config
  }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

/// Test that error contexts include message information
#[test]
fn test_error_context_includes_message_id() {
  let producer = FailingProducer::new(vec![1, 2, 3]);
  let msg = wrap_message(42);
  let msg_id = msg.id().clone();

  let context = producer.create_error_context(Some(msg));

  assert!(context.item.is_some());
  let context_msg = context.item.unwrap();
  assert_eq!(
    context_msg.id(),
    &msg_id,
    "Error context should include message ID"
  );
}

/// Test that error contexts include message metadata
#[test]
fn test_error_context_includes_message_metadata() {
  let producer = FailingProducer::new(vec![1, 2, 3]);
  let metadata = MessageMetadata::default()
    .source("test_source")
    .header("key", "value");
  let msg = Message::with_metadata(42, MessageId::new_uuid(), metadata.clone());

  let context = producer.create_error_context(Some(msg));

  assert!(context.item.is_some());
  let context_msg = context.item.unwrap();
  assert_eq!(
    context_msg.metadata().get_source(),
    metadata.get_source(),
    "Error context should include message metadata"
  );
  assert_eq!(
    context_msg.metadata().get_header("key"),
    metadata.get_header("key"),
    "Error context should include message headers"
  );
}

/// Test error handling with Stop strategy
#[test]
fn test_error_handling_stop_strategy() {
  let producer = FailingProducer::new(vec![1, 2, 3]).with_config(
    streamweave::ProducerConfig::default().with_error_strategy(ErrorStrategy::<Message<i32>>::Stop),
  );

  let msg = wrap_message(42);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: producer.create_error_context(Some(msg)),
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "FailingProducer".to_string(),
    },
    retries: 0,
  };

  assert!(
    matches!(producer.handle_error(&error), ErrorAction::Stop),
    "Stop strategy should return Stop action"
  );
}

/// Test error handling with Skip strategy
#[test]
fn test_error_handling_skip_strategy() {
  let producer = FailingProducer::new(vec![1, 2, 3]).with_config(
    streamweave::ProducerConfig::default().with_error_strategy(ErrorStrategy::<Message<i32>>::Skip),
  );

  let msg = wrap_message(42);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: producer.create_error_context(Some(msg)),
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "FailingProducer".to_string(),
    },
    retries: 0,
  };

  assert!(
    matches!(producer.handle_error(&error), ErrorAction::Skip),
    "Skip strategy should return Skip action"
  );
}

/// Test error handling with Retry strategy
#[test]
fn test_error_handling_retry_strategy() {
  let producer = FailingProducer::new(vec![1, 2, 3]).with_config(
    streamweave::ProducerConfig::default()
      .with_error_strategy(ErrorStrategy::<Message<i32>>::Retry(3)),
  );

  let msg = wrap_message(42);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: producer.create_error_context(Some(msg)),
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "FailingProducer".to_string(),
    },
    retries: 1, // Below limit
  };

  assert!(
    matches!(producer.handle_error(&error), ErrorAction::Retry),
    "Retry strategy should return Retry action when below limit"
  );
}

/// Test error handling with Retry strategy exhausted
#[test]
fn test_error_handling_retry_exhausted() {
  let producer = FailingProducer::new(vec![1, 2, 3]).with_config(
    streamweave::ProducerConfig::default()
      .with_error_strategy(ErrorStrategy::<Message<i32>>::Retry(3)),
  );

  let msg = wrap_message(42);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: producer.create_error_context(Some(msg)),
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "FailingProducer".to_string(),
    },
    retries: 3, // At limit
  };

  assert!(
    matches!(producer.handle_error(&error), ErrorAction::Stop),
    "Retry strategy should return Stop action when retries exhausted"
  );
}

/// Test error handling with Custom strategy
#[test]
fn test_error_handling_custom_strategy() {
  let custom_handler = |error: &StreamError<Message<i32>>| {
    // Check if message payload is even
    if let Some(msg) = &error.context.item {
      if *msg.payload() % 2 == 0 {
        ErrorAction::Skip
      } else {
        ErrorAction::Stop
      }
    } else {
      ErrorAction::Stop
    }
  };

  let producer = FailingProducer::new(vec![1, 2, 3]).with_config(
    streamweave::ProducerConfig::default().with_error_strategy(
      ErrorStrategy::<Message<i32>>::Custom(Arc::new(custom_handler)),
    ),
  );

  // Test with even payload
  let msg_even = wrap_message(42);
  let error_even = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: producer.create_error_context(Some(msg_even)),
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "FailingProducer".to_string(),
    },
    retries: 0,
  };

  assert!(
    matches!(producer.handle_error(&error_even), ErrorAction::Skip),
    "Custom strategy should skip even payloads"
  );

  // Test with odd payload
  let msg_odd = wrap_message(41);
  let error_odd = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: producer.create_error_context(Some(msg_odd)),
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "FailingProducer".to_string(),
    },
    retries: 0,
  };

  assert!(
    matches!(producer.handle_error(&error_odd), ErrorAction::Stop),
    "Custom strategy should stop on odd payloads"
  );
}

/// Test that error context preserves message integrity
#[test]
fn test_error_context_preserves_message_integrity() {
  let producer = FailingProducer::new(vec![1, 2, 3]);
  let original_id = MessageId::new_uuid();
  let original_metadata = MessageMetadata::default()
    .source("original_source")
    .header("original_key", "original_value");
  let original_payload = 42;

  let msg = Message::with_metadata(
    original_payload,
    original_id.clone(),
    original_metadata.clone(),
  );
  let context = producer.create_error_context(Some(msg));

  assert!(context.item.is_some());
  let context_msg = context.item.unwrap();

  // Verify all message components are preserved
  assert_eq!(
    context_msg.id(),
    &original_id,
    "Message ID should be preserved in error context"
  );
  assert_eq!(
    *context_msg.payload(),
    original_payload,
    "Message payload should be preserved in error context"
  );
  assert_eq!(
    context_msg.metadata().get_source(),
    original_metadata.get_source(),
    "Message metadata should be preserved in error context"
  );
  assert_eq!(
    context_msg.metadata().get_header("original_key"),
    original_metadata.get_header("original_key"),
    "Message headers should be preserved in error context"
  );
}

/// Test error context with None item
#[test]
fn test_error_context_with_none_item() {
  let producer = FailingProducer::new(vec![1, 2, 3]);
  let context = producer.create_error_context(None);

  assert!(
    context.item.is_none(),
    "Error context should handle None item"
  );
  assert_eq!(
    context.component_name,
    producer.component_info().name,
    "Error context should include component name"
  );
  assert_eq!(
    context.component_type,
    producer.component_info().type_name,
    "Error context should include component type"
  );
}

/// Test transformer error handling with messages
#[test]
fn test_transformer_error_handling_with_messages() {
  let transformer = FailingTransformer::new().with_config(
    streamweave::TransformerConfig::default()
      .with_error_strategy(ErrorStrategy::<Message<i32>>::Skip),
  );

  let msg = wrap_message(42);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: transformer.create_error_context(Some(msg)),
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "FailingTransformer".to_string(),
    },
    retries: 0,
  };

  assert!(
    matches!(transformer.handle_error(&error), ErrorAction::Skip),
    "Transformer should handle errors with Skip strategy"
  );
}

/// Test consumer error handling with messages
#[test]
fn test_consumer_error_handling_with_messages() {
  let mut consumer = CollectingConsumer::new();
  let config = streamweave::ConsumerConfig {
    error_strategy: ErrorStrategy::<Message<i32>>::Stop,
    ..Default::default()
  };
  consumer.set_config(config);

  let msg = wrap_message(42);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: consumer.create_error_context(Some(msg)),
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "CollectingConsumer".to_string(),
    },
    retries: 0,
  };

  assert!(
    matches!(consumer.handle_error(&error), ErrorAction::Stop),
    "Consumer should handle errors with Stop strategy"
  );
}

/// Test that error context includes component information
#[test]
fn test_error_context_includes_component_info() {
  let mut producer = FailingProducer::new(vec![1, 2, 3]);
  let config = streamweave::ProducerConfig {
    name: Some("test_producer".to_string()),
    ..Default::default()
  };
  producer.set_config(config);
  let msg = wrap_message(42);
  let context = producer.create_error_context(Some(msg));

  assert_eq!(
    context.component_name, "test_producer",
    "Error context should include component name"
  );
  assert!(
    context.component_type.contains("FailingProducer"),
    "Error context should include component type"
  );
}

/// Test error context timestamp
#[test]
fn test_error_context_timestamp() {
  let producer = FailingProducer::new(vec![1, 2, 3]);
  let msg = wrap_message(42);

  let before = chrono::Utc::now();
  let context = producer.create_error_context(Some(msg));
  let after = chrono::Utc::now();

  assert!(
    context.timestamp >= before,
    "Error context timestamp should be after 'before' time"
  );
  assert!(
    context.timestamp <= after,
    "Error context timestamp should be before 'after' time"
  );
}

/// Test that custom error handler can access message information
#[test]
fn test_custom_error_handler_accesses_message_info() {
  let custom_handler = |error: &StreamError<Message<i32>>| {
    if let Some(msg) = &error.context.item {
      // Access message ID
      let _id = msg.id();
      // Access message payload
      let payload = *msg.payload();
      // Access message metadata
      let _source = msg.metadata().get_source();

      // Make decision based on message content
      if payload > 50 {
        ErrorAction::Retry
      } else {
        ErrorAction::Skip
      }
    } else {
      ErrorAction::Stop
    }
  };

  let producer = FailingProducer::new(vec![1, 2, 3]).with_config(
    streamweave::ProducerConfig::default().with_error_strategy(
      ErrorStrategy::<Message<i32>>::Custom(Arc::new(custom_handler)),
    ),
  );

  // Test with payload > 50
  let msg_high = wrap_message(100);
  let error_high = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: producer.create_error_context(Some(msg_high)),
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "FailingProducer".to_string(),
    },
    retries: 0,
  };

  assert!(
    matches!(producer.handle_error(&error_high), ErrorAction::Retry),
    "Custom handler should retry for payload > 50"
  );

  // Test with payload <= 50
  let msg_low = wrap_message(30);
  let error_low = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: producer.create_error_context(Some(msg_low)),
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "FailingProducer".to_string(),
    },
    retries: 0,
  };

  assert!(
    matches!(producer.handle_error(&error_low), ErrorAction::Skip),
    "Custom handler should skip for payload <= 50"
  );
}
