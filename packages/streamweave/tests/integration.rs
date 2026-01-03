//! Integration tests for full Producer → Transformer → Consumer pipeline with Message<T>
//!
//! These tests verify that messages flow correctly through a complete pipeline,
//! that metadata is preserved, and that IDs are maintained throughout.

use futures::StreamExt;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::message::{Message, MessageId, MessageMetadata, wrap_message};
use streamweave::{Consumer, Input, Output, Producer, Transformer};
use tokio::sync::Mutex;
use tokio_stream::Stream;

// ============================================================================
// Test Components
// ============================================================================

/// Test producer that yields Message<i32> from a vector
#[derive(Clone)]
struct TestProducer {
  items: Vec<i32>,
  config: streamweave::ProducerConfig<Message<i32>>,
}

impl TestProducer {
  fn new(items: Vec<i32>) -> Self {
    Self {
      items,
      config: streamweave::ProducerConfig::default(),
    }
  }
}

impl Output for TestProducer {
  type Output = Message<i32>;
  type OutputStream = Pin<Box<dyn Stream<Item = Message<i32>> + Send>>;
}

#[async_trait::async_trait]
impl Producer for TestProducer {
  type OutputPorts = (Message<i32>,);

  fn produce(&mut self) -> Self::OutputStream {
    let items = self.items.clone();
    Box::pin(futures::stream::iter(items.into_iter().map(wrap_message)))
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

/// Test transformer that doubles the payload value
struct DoubleTransformer {
  config: streamweave::TransformerConfig<Message<i32>>,
}

impl DoubleTransformer {
  fn new() -> Self {
    Self {
      config: streamweave::TransformerConfig::default(),
    }
  }
}

impl Input for DoubleTransformer {
  type Input = Message<i32>;
  type InputStream = Pin<Box<dyn Stream<Item = Message<i32>> + Send>>;
}

impl Output for DoubleTransformer {
  type Output = Message<i32>;
  type OutputStream = Pin<Box<dyn Stream<Item = Message<i32>> + Send>>;
}

#[async_trait::async_trait]
impl Transformer for DoubleTransformer {
  type InputPorts = (Message<i32>,);
  type OutputPorts = (Message<i32>,);

  async fn transform(&mut self, stream: Self::InputStream) -> Self::OutputStream {
    Box::pin(stream.map(|msg| {
      let payload = *msg.payload();
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

/// Test consumer that collects messages
struct CollectConsumer {
  items: Arc<Mutex<Vec<Message<i32>>>>,
  config: streamweave::ConsumerConfig<Message<i32>>,
}

impl CollectConsumer {
  fn new() -> Self {
    Self {
      items: Arc::new(Mutex::new(Vec::new())),
      config: streamweave::ConsumerConfig::default(),
    }
  }

  fn into_inner(self) -> Arc<Mutex<Vec<Message<i32>>>> {
    self.items
  }
}

impl Input for CollectConsumer {
  type Input = Message<i32>;
  type InputStream = Pin<Box<dyn Stream<Item = Message<i32>> + Send>>;
}

#[async_trait::async_trait]
impl Consumer for CollectConsumer {
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
// Integration Tests
// ============================================================================

/// Test that messages flow correctly through a full pipeline
#[tokio::test]
async fn test_full_pipeline_message_flow() {
  let mut producer = TestProducer::new(vec![1, 2, 3, 4, 5]);
  let mut transformer = DoubleTransformer::new();
  let mut consumer = CollectConsumer::new();

  // Connect producer → transformer → consumer
  let producer_stream = producer.produce();
  let transformer_stream = transformer.transform(Box::pin(producer_stream)).await;
  consumer.consume(Box::pin(transformer_stream)).await;

  // Verify results
  let items = consumer.into_inner();
  let messages = items.lock().await;
  assert_eq!(messages.len(), 5);

  // Verify payloads are doubled
  let payloads: Vec<i32> = messages.iter().map(|msg| *msg.payload()).collect();
  assert_eq!(payloads, vec![2, 4, 6, 8, 10]);
}

/// Test that message IDs are preserved through the pipeline
#[tokio::test]
async fn test_pipeline_preserves_message_ids() {
  // Create messages with known IDs
  let message_ids: Vec<MessageId> = (0..3).map(|_| MessageId::new_uuid()).collect();
  let messages: Vec<Message<i32>> = (0..3)
    .map(|i| {
      Message::with_metadata(
        (i + 1) as i32,
        message_ids[i].clone(),
        MessageMetadata::default(),
      )
    })
    .collect();

  // Create a producer that yields these specific messages
  struct IdPreservingProducer {
    messages: Vec<Message<i32>>,
    config: streamweave::ProducerConfig<Message<i32>>,
  }

  impl IdPreservingProducer {
    fn new(messages: Vec<Message<i32>>) -> Self {
      Self {
        messages,
        config: streamweave::ProducerConfig::default(),
      }
    }
  }

  impl Output for IdPreservingProducer {
    type Output = Message<i32>;
    type OutputStream = Pin<Box<dyn Stream<Item = Message<i32>> + Send>>;
  }

  #[async_trait::async_trait]
  impl Producer for IdPreservingProducer {
    type OutputPorts = (Message<i32>,);

    fn produce(&mut self) -> Self::OutputStream {
      let messages = self.messages.clone();
      Box::pin(futures::stream::iter(messages))
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

  let mut producer = IdPreservingProducer::new(messages);
  let mut transformer = DoubleTransformer::new();
  let mut consumer = CollectConsumer::new();

  // Run the full pipeline
  let producer_stream = producer.produce();
  let transformer_stream = transformer.transform(Box::pin(producer_stream)).await;
  consumer.consume(Box::pin(transformer_stream)).await;

  // Verify IDs are preserved
  let items = consumer.into_inner();
  let output_messages = items.lock().await;
  assert_eq!(output_messages.len(), 3);

  for (i, msg) in output_messages.iter().enumerate() {
    assert_eq!(
      msg.id(),
      &message_ids[i],
      "Message ID should be preserved through the pipeline"
    );
  }
}

/// Test that message metadata is preserved through the pipeline
#[tokio::test]
async fn test_pipeline_preserves_metadata() {
  // Create messages with custom metadata
  let mut messages = Vec::new();
  for i in 1..=3 {
    let metadata = MessageMetadata::default()
      .source("test")
      .header("index", i.to_string());
    messages.push(Message::with_metadata(i, MessageId::new_uuid(), metadata));
  }

  // Create a producer that yields these messages
  struct CustomProducer {
    messages: Vec<Message<i32>>,
    config: streamweave::ProducerConfig<Message<i32>>,
  }

  impl CustomProducer {
    fn new(messages: Vec<Message<i32>>) -> Self {
      Self {
        messages,
        config: streamweave::ProducerConfig::default(),
      }
    }
  }

  impl Output for CustomProducer {
    type Output = Message<i32>;
    type OutputStream = Pin<Box<dyn Stream<Item = Message<i32>> + Send>>;
  }

  #[async_trait::async_trait]
  impl Producer for CustomProducer {
    type OutputPorts = (Message<i32>,);

    fn produce(&mut self) -> Self::OutputStream {
      let messages = self.messages.clone();
      Box::pin(futures::stream::iter(messages))
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

  let mut producer = CustomProducer::new(messages);
  let mut transformer = DoubleTransformer::new();
  let mut consumer = CollectConsumer::new();

  // Run pipeline
  let producer_stream = producer.produce();
  let transformer_stream = transformer.transform(Box::pin(producer_stream)).await;
  consumer.consume(Box::pin(transformer_stream)).await;

  // Verify metadata is preserved
  let items = consumer.into_inner();
  let output_messages = items.lock().await;
  assert_eq!(output_messages.len(), 3);

  for (i, msg) in output_messages.iter().enumerate() {
    let metadata = msg.metadata();
    assert_eq!(
      metadata.get_source(),
      Some("test"),
      "Source metadata should be preserved"
    );
    assert_eq!(
      metadata.get_header("index"),
      Some((i + 1).to_string().as_str()),
      "Index metadata should be preserved"
    );
  }
}

/// Test pipeline with multiple transformers in sequence
#[tokio::test]
async fn test_pipeline_multiple_transformers() {
  /// Transformer that adds 1 to the payload
  struct AddOneTransformer {
    config: streamweave::TransformerConfig<Message<i32>>,
  }

  impl AddOneTransformer {
    fn new() -> Self {
      Self {
        config: streamweave::TransformerConfig::default(),
      }
    }
  }

  impl Input for AddOneTransformer {
    type Input = Message<i32>;
    type InputStream = Pin<Box<dyn Stream<Item = Message<i32>> + Send>>;
  }

  impl Output for AddOneTransformer {
    type Output = Message<i32>;
    type OutputStream = Pin<Box<dyn Stream<Item = Message<i32>> + Send>>;
  }

  #[async_trait::async_trait]
  impl Transformer for AddOneTransformer {
    type InputPorts = (Message<i32>,);
    type OutputPorts = (Message<i32>,);

    async fn transform(&mut self, stream: Self::InputStream) -> Self::OutputStream {
      Box::pin(stream.map(|msg| {
        let payload = *msg.payload();
        let id = msg.id().clone();
        let metadata = msg.metadata().clone();
        Message::with_metadata(payload + 1, id, metadata)
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

  let mut producer = TestProducer::new(vec![1, 2, 3]);
  let mut transformer1 = DoubleTransformer::new(); // 1,2,3 -> 2,4,6
  let mut transformer2 = AddOneTransformer::new(); // 2,4,6 -> 3,5,7
  let mut consumer = CollectConsumer::new();

  // Connect producer → transformer1 → transformer2 → consumer
  let producer_stream = producer.produce();
  let transformer1_stream = transformer1.transform(Box::pin(producer_stream)).await;
  let transformer2_stream = transformer2.transform(transformer1_stream).await;
  consumer.consume(Box::pin(transformer2_stream)).await;

  // Verify results: (1*2+1, 2*2+1, 3*2+1) = (3, 5, 7)
  let items = consumer.into_inner();
  let messages = items.lock().await;
  assert_eq!(messages.len(), 3);

  let payloads: Vec<i32> = messages.iter().map(|msg| *msg.payload()).collect();
  assert_eq!(payloads, vec![3, 5, 7]);
}

/// Test pipeline with empty input
#[tokio::test]
async fn test_pipeline_empty_input() {
  let mut producer = TestProducer::new(vec![]);
  let mut transformer = DoubleTransformer::new();
  let mut consumer = CollectConsumer::new();

  let producer_stream = producer.produce();
  let transformer_stream = transformer.transform(Box::pin(producer_stream)).await;
  consumer.consume(Box::pin(transformer_stream)).await;

  let items = consumer.into_inner();
  let messages = items.lock().await;
  assert_eq!(messages.len(), 0);
}

/// Test pipeline preserves message order
#[tokio::test]
async fn test_pipeline_preserves_order() {
  let input = vec![10, 20, 30, 40, 50];
  let mut producer = TestProducer::new(input.clone());
  let mut transformer = DoubleTransformer::new();
  let mut consumer = CollectConsumer::new();

  let producer_stream = producer.produce();
  let transformer_stream = transformer.transform(Box::pin(producer_stream)).await;
  consumer.consume(Box::pin(transformer_stream)).await;

  let items = consumer.into_inner();
  let messages = items.lock().await;
  assert_eq!(messages.len(), 5);

  // Verify order is preserved
  for (i, msg) in messages.iter().enumerate() {
    let expected = input[i] * 2;
    assert_eq!(
      msg.payload(),
      &expected,
      "Message order should be preserved"
    );
  }
}

/// Test that each message in the pipeline has a unique ID
#[tokio::test]
async fn test_pipeline_unique_message_ids() {
  let mut producer = TestProducer::new(vec![1, 2, 3, 4, 5]);
  let mut transformer = DoubleTransformer::new();
  let mut consumer = CollectConsumer::new();

  let producer_stream = producer.produce();
  let transformer_stream = transformer.transform(Box::pin(producer_stream)).await;
  consumer.consume(Box::pin(transformer_stream)).await;

  let items = consumer.into_inner();
  let messages = items.lock().await;
  assert_eq!(messages.len(), 5);

  // Verify all IDs are unique
  let mut ids = std::collections::HashSet::new();
  for msg in messages.iter() {
    assert!(
      ids.insert(msg.id().clone()),
      "Each message should have a unique ID"
    );
  }
}
