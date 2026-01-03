//! Tests for Producer trait with Message<T>

use futures::StreamExt;
use std::pin::Pin;
use streamweave::message::{Message, MessageId, MessageMetadata, wrap_message};
use streamweave::{Output, Producer, ProducerConfig, ProducerPorts};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tokio_stream::Stream;

// Test error type
#[derive(Debug)]
struct TestError(String);

impl std::fmt::Display for TestError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl std::error::Error for TestError {}

// Test producer that yields Message<T> from a vector
#[derive(Clone)]
struct TestProducer<T: std::fmt::Debug + Clone + Send + Sync> {
  items: Vec<T>,
  config: ProducerConfig<Message<T>>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync> TestProducer<T> {
  fn new(items: Vec<T>) -> Self {
    Self {
      items,
      config: ProducerConfig::default(),
    }
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for TestProducer<T> {
  type Output = Message<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Message<T>> + Send>>;
}

#[async_trait::async_trait]
impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Producer for TestProducer<T> {
  type OutputPorts = (Message<T>,);

  fn produce(&mut self) -> Self::OutputStream {
    let items = self.items.clone();
    Box::pin(futures::stream::iter(
      items.into_iter().map(|item| wrap_message(item)),
    ))
  }

  fn set_config_impl(&mut self, config: ProducerConfig<Message<T>>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<Message<T>> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<Message<T>> {
    &mut self.config
  }
}

#[tokio::test]
async fn test_producer_with_messages() {
  let mut producer = TestProducer::new(vec![1, 2, 3]);
  let stream = producer.produce();
  let messages: Vec<Message<i32>> = stream.collect().await;

  assert_eq!(messages.len(), 3);
  assert_eq!(*messages[0].payload(), 1);
  assert_eq!(*messages[1].payload(), 2);
  assert_eq!(*messages[2].payload(), 3);

  // Verify all messages have IDs
  for msg in &messages {
    assert!(msg.id().is_uuid());
  }
}

#[tokio::test]
async fn test_producer_message_ids_unique() {
  let mut producer = TestProducer::new(vec![1, 2, 3]);
  let stream = producer.produce();
  let messages: Vec<Message<i32>> = stream.collect().await;

  // All IDs should be unique
  let ids: Vec<MessageId> = messages.iter().map(|m| m.id().clone()).collect();
  for i in 0..ids.len() {
    for j in (i + 1)..ids.len() {
      assert_ne!(ids[i], ids[j]);
    }
  }
}

#[tokio::test]
async fn test_producer_message_metadata() {
  let mut producer = TestProducer::new(vec![1, 2, 3]);
  let stream = producer.produce();
  let messages: Vec<Message<i32>> = stream.collect().await;

  // All messages should have metadata with timestamps
  for msg in &messages {
    assert!(msg.metadata().timestamp.is_some());
  }
}

#[test]
fn test_producer_config() {
  let producer = TestProducer::new(vec![1, 2, 3]).with_config(
    ProducerConfig::default()
      .with_name("test_producer".to_string())
      .with_error_strategy(ErrorStrategy::Skip),
  );
  assert_eq!(producer.config().name(), Some("test_producer".to_string()));
  assert!(matches!(
    producer.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[test]
fn test_producer_error_handling_with_messages() {
  let producer = TestProducer::new(vec![1, 2, 3])
    .with_config(ProducerConfig::default().with_error_strategy(ErrorStrategy::Skip));
  let msg = wrap_message(42);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(msg),
      component_name: "test".to_string(),
      component_type: "TestProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "TestProducer".to_string(),
    },
    retries: 0,
  };
  assert!(matches!(producer.handle_error(&error), ErrorAction::Skip));
}

#[tokio::test]
async fn test_empty_producer() {
  let mut producer = TestProducer::new(Vec::<i32>::new());
  let stream = producer.produce();
  let messages: Vec<Message<i32>> = stream.collect().await;
  assert!(messages.is_empty());
}

#[test]
fn test_different_error_strategies() {
  let producer = TestProducer::new(vec![1, 2, 3])
    .with_config(ProducerConfig::default().with_error_strategy(ErrorStrategy::Stop));
  let msg = wrap_message(42);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(msg),
      component_name: "test".to_string(),
      component_type: "TestProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "TestProducer".to_string(),
    },
    retries: 0,
  };
  assert!(matches!(producer.handle_error(&error), ErrorAction::Stop));

  let producer =
    producer.with_config(ProducerConfig::default().with_error_strategy(ErrorStrategy::Retry(3)));
  assert!(matches!(producer.handle_error(&error), ErrorAction::Retry));
}

#[test]
fn test_component_info() {
  let producer = TestProducer::new(vec![1, 2, 3]).with_name("test_producer".to_string());

  let info = producer.component_info();
  assert_eq!(info.name, "test_producer");
  assert!(info.type_name.contains("TestProducer"));
}

#[test]
fn test_error_context_creation_with_message() {
  let producer = TestProducer::new(vec![1, 2, 3]).with_name("test_producer".to_string());
  let msg = wrap_message(42);

  let context = producer.create_error_context(Some(msg.clone()));
  assert_eq!(context.component_name, "test_producer");
  assert!(context.component_type.contains("TestProducer"));
  assert_eq!(context.item, Some(msg));

  // Verify message ID is accessible from error context
  if let Some(error_msg) = &context.item {
    assert!(error_msg.id().is_uuid());
    assert_eq!(*error_msg.payload(), 42);
  }
}

#[test]
fn test_error_context_message_metadata() {
  let producer = TestProducer::new(vec![1, 2, 3]).with_name("test_producer".to_string());
  let metadata = MessageMetadata::new().source("test-source");
  let msg = Message::with_metadata(42, MessageId::new_uuid(), metadata);

  let context = producer.create_error_context(Some(msg.clone()));

  // Verify message metadata is accessible from error context
  if let Some(error_msg) = &context.item {
    assert_eq!(error_msg.metadata().get_source(), Some("test-source"));
  }
}

#[test]
fn test_with_name() {
  let producer = TestProducer::new(vec![1, 2, 3]).with_name("test_producer".to_string());

  assert_eq!(producer.config().name(), Some("test_producer".to_string()));
}

#[test]
fn test_config_mut() {
  let mut producer = TestProducer::new(vec![1, 2, 3]);
  producer.config_mut().name = Some("test_producer".to_string());
  assert_eq!(producer.config().name(), Some("test_producer".to_string()));
}

#[tokio::test]
async fn test_producer_with_strings() {
  let mut producer = TestProducer::new(vec!["hello".to_string(), "world".to_string()]);
  let stream = producer.produce();
  let messages: Vec<Message<String>> = stream.collect().await;

  assert_eq!(messages.len(), 2);
  assert_eq!(*messages[0].payload(), "hello".to_string());
  assert_eq!(*messages[1].payload(), "world".to_string());
}

#[test]
fn test_producer_config_default() {
  let config = ProducerConfig::<Message<i32>>::default();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Stop));
  assert_eq!(config.name(), None);
}

#[test]
fn test_producer_config_with_error_strategy() {
  let config = ProducerConfig::<Message<i32>>::default()
    .with_error_strategy(ErrorStrategy::<Message<i32>>::Skip);
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));

  let config = ProducerConfig::<Message<i32>>::default()
    .with_error_strategy(ErrorStrategy::<Message<i32>>::Retry(5));
  assert!(matches!(config.error_strategy(), ErrorStrategy::Retry(5)));
}

#[test]
fn test_producer_config_with_name() {
  let config = ProducerConfig::<Message<i32>>::default().with_name("test_producer".to_string());
  assert_eq!(config.name(), Some("test_producer".to_string()));
}

#[test]
fn test_producer_config_error_strategy() {
  let config = ProducerConfig::<Message<i32>>::default()
    .with_error_strategy(ErrorStrategy::<Message<i32>>::Skip);
  let strategy = config.error_strategy();
  assert!(matches!(strategy, ErrorStrategy::Skip));
}

#[test]
fn test_producer_config_name() {
  let config = ProducerConfig::<Message<i32>>::default().with_name("test_name".to_string());
  assert_eq!(config.name(), Some("test_name".to_string()));

  let config = ProducerConfig::<Message<i32>>::default();
  assert_eq!(config.name(), None);
}

#[test]
fn test_producer_handle_error_custom() {
  let producer =
    TestProducer::new(vec![1, 2, 3]).with_config(ProducerConfig::default().with_error_strategy(
      ErrorStrategy::<Message<i32>>::new_custom(|error: &StreamError<Message<i32>>| {
        if error.retries < 2 {
          ErrorAction::Retry
        } else {
          ErrorAction::Skip
        }
      }),
    ));

  let msg = wrap_message(42);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(msg),
      component_name: "test".to_string(),
      component_type: "TestProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "TestProducer".to_string(),
    },
    retries: 1,
  };

  assert!(matches!(producer.handle_error(&error), ErrorAction::Retry));

  let msg2 = wrap_message(43);
  let error_exhausted = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(msg2),
      component_name: "test".to_string(),
      component_type: "TestProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "TestProducer".to_string(),
    },
    retries: 2,
  };

  assert!(matches!(
    producer.handle_error(&error_exhausted),
    ErrorAction::Skip
  ));
}

#[test]
fn test_producer_handle_error_retry_exhausted() {
  let producer = TestProducer::new(vec![1, 2, 3]).with_config(
    ProducerConfig::default().with_error_strategy(ErrorStrategy::<Message<i32>>::Retry(3)),
  );

  let msg = wrap_message(42);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(msg),
      component_name: "test".to_string(),
      component_type: "TestProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "TestProducer".to_string(),
    },
    retries: 3,
  };

  assert!(matches!(producer.handle_error(&error), ErrorAction::Stop));
}

#[test]
fn test_producer_create_error_context_with_none() {
  let producer = TestProducer::new(vec![1, 2, 3]).with_name("test_producer".to_string());

  let context = producer.create_error_context(None);
  assert_eq!(context.component_name, "test_producer");
  assert!(context.component_type.contains("TestProducer"));
  assert_eq!(context.item, None);
}

#[test]
fn test_producer_component_info_default_name() {
  let producer = TestProducer::new(vec![1, 2, 3]);

  let info = producer.component_info();
  assert_eq!(info.name, "producer");
  assert!(info.type_name.contains("TestProducer"));
}

#[test]
fn test_producer_with_config() {
  let producer1 = TestProducer::new(vec![1, 2, 3]);
  let config = ProducerConfig::default()
    .with_name("new_name".to_string())
    .with_error_strategy(ErrorStrategy::<Message<i32>>::Skip);

  let producer2 = producer1.with_config(config);
  assert_eq!(producer2.config().name(), Some("new_name".to_string()));
  assert!(matches!(
    producer2.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[test]
fn test_producer_set_config() {
  let mut producer = TestProducer::new(vec![1, 2, 3]);
  let config = ProducerConfig::default()
    .with_name("set_name".to_string())
    .with_error_strategy(ErrorStrategy::<Message<i32>>::Retry(5));

  producer.set_config(config);
  assert_eq!(producer.config().name(), Some("set_name".to_string()));
  assert!(matches!(
    producer.config().error_strategy(),
    ErrorStrategy::Retry(5)
  ));
}

#[test]
fn test_producer_config_clone() {
  let config1 = ProducerConfig::default()
    .with_name("test".to_string())
    .with_error_strategy(ErrorStrategy::<Message<i32>>::Skip);
  let config2 = config1.clone();

  assert_eq!(config1.name(), config2.name());
  assert!(matches!(config1.error_strategy(), ErrorStrategy::Skip));
  assert!(matches!(config2.error_strategy(), ErrorStrategy::Skip));
}

#[test]
fn test_producer_config_debug() {
  let config = ProducerConfig::<Message<i32>>::default().with_name("test".to_string());
  let debug_str = format!("{:?}", config);
  assert!(!debug_str.is_empty());
}

#[test]
fn test_producer_ports_trait() {
  // Test that ProducerPorts trait provides default output ports
  // This test verifies the blanket implementation works by checking the type exists
  fn _test_producer_ports<P: Producer + ProducerPorts>()
  where
    P::Output: std::fmt::Debug + Clone + Send + Sync,
  {
    // Just verify the associated type exists - don't try to assign
    let _phantom: std::marker::PhantomData<<P as ProducerPorts>::DefaultOutputPorts> =
      std::marker::PhantomData;
  }

  // This test verifies the blanket implementation works
  let _producer = TestProducer::new(vec![1, 2, 3]);
  // The trait is implemented via blanket impl, so this test verifies it compiles
}

#[test]
fn test_producer_set_config_directly() {
  let mut producer = TestProducer::new(vec![1, 2, 3]);
  let config = ProducerConfig::default()
    .with_name("direct_set".to_string())
    .with_error_strategy(ErrorStrategy::<Message<i32>>::Retry(10));

  producer.set_config(config);
  assert_eq!(producer.config().name(), Some("direct_set".to_string()));
  assert!(matches!(
    producer.config().error_strategy(),
    ErrorStrategy::Retry(10)
  ));
}

#[test]
fn test_producer_config_mut_modification() {
  let mut producer = TestProducer::new(vec![1, 2, 3]);
  let config_mut = producer.config_mut();
  config_mut.name = Some("mutated".to_string());
  config_mut.error_strategy = ErrorStrategy::<Message<i32>>::Skip;

  assert_eq!(producer.config().name(), Some("mutated".to_string()));
  assert!(matches!(
    producer.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[test]
fn test_producer_handle_error_retry_at_limit() {
  let producer = TestProducer::new(vec![1, 2, 3]).with_config(
    ProducerConfig::default().with_error_strategy(ErrorStrategy::<Message<i32>>::Retry(3)),
  );

  let msg = wrap_message(42);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(msg),
      component_name: "test".to_string(),
      component_type: "TestProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "TestProducer".to_string(),
    },
    retries: 3, // Exactly at limit
  };

  // Should stop when retries equals limit
  assert!(matches!(producer.handle_error(&error), ErrorAction::Stop));
}

#[test]
fn test_producer_handle_error_retry_below_limit() {
  let producer = TestProducer::new(vec![1, 2, 3]).with_config(
    ProducerConfig::default().with_error_strategy(ErrorStrategy::<Message<i32>>::Retry(5)),
  );

  let msg = wrap_message(42);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(msg),
      component_name: "test".to_string(),
      component_type: "TestProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "TestProducer".to_string(),
    },
    retries: 2, // Below limit
  };

  assert!(matches!(producer.handle_error(&error), ErrorAction::Retry));
}

#[test]
fn test_producer_create_error_context_timestamp() {
  let producer = TestProducer::new(vec![1, 2, 3]).with_name("test_producer".to_string());
  let msg = wrap_message(42);

  let before = chrono::Utc::now();
  let context = producer.create_error_context(Some(msg.clone()));
  let after = chrono::Utc::now();

  assert!(context.timestamp >= before);
  assert!(context.timestamp <= after);
  assert_eq!(context.item, Some(msg));
}

#[tokio::test]
async fn test_producer_produce_multiple_times() {
  let mut producer = TestProducer::new(vec![1, 2, 3]);

  // First production
  let stream1 = producer.produce();
  let messages1: Vec<Message<i32>> = stream1.collect().await;
  assert_eq!(messages1.len(), 3);
  assert_eq!(*messages1[0].payload(), 1);

  // Second production (should work again)
  let stream2 = producer.produce();
  let messages2: Vec<Message<i32>> = stream2.collect().await;
  assert_eq!(messages2.len(), 3);
  assert_eq!(*messages2[0].payload(), 1);

  // IDs should be different between productions
  assert_ne!(messages1[0].id(), messages2[0].id());
}

#[test]
fn test_producer_with_name_preserves_error_strategy() {
  let producer = TestProducer::new(vec![1, 2, 3]).with_config(
    ProducerConfig::default().with_error_strategy(ErrorStrategy::<Message<i32>>::Skip),
  );

  let producer = producer.with_name("named_producer".to_string());

  assert_eq!(producer.config().name(), Some("named_producer".to_string()));
  assert!(matches!(
    producer.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_producer_message_payload_extraction() {
  let mut producer = TestProducer::new(vec![1, 2, 3]);
  let stream = producer.produce();
  let messages: Vec<Message<i32>> = stream.collect().await;

  // Extract payloads
  let payloads: Vec<i32> = messages.iter().map(|m| *m.payload()).collect();
  assert_eq!(payloads, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_producer_message_id_preservation() {
  let mut producer = TestProducer::new(vec![1, 2, 3]);
  let stream = producer.produce();
  let messages: Vec<Message<i32>> = stream.collect().await;

  // Store IDs
  let ids: Vec<MessageId> = messages.iter().map(|m| m.id().clone()).collect();

  // Verify IDs are preserved when accessing messages
  for (i, msg) in messages.iter().enumerate() {
    assert_eq!(msg.id(), &ids[i]);
  }
}
