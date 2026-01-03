//! Tests for adapters module

use futures::StreamExt;
use std::sync::Arc;
use streamweave::adapters::*;
use streamweave::consumers::VecConsumer;
use streamweave::message::{Message, MessageId, UuidGenerator};
use streamweave::producers::VecProducer;
use streamweave::transformers::MapTransformer;

struct TestRawProducer {
  items: Vec<i32>,
}

impl RawProducer for TestRawProducer {
  type Payload = i32;

  fn produce_raw(&mut self) -> std::pin::Pin<Box<dyn futures::Stream<Item = i32> + Send>> {
    Box::pin(futures::stream::iter(self.items.clone()))
  }
}

#[tokio::test]
async fn test_message_wrapper_new() {
  let raw_producer = TestRawProducer {
    items: vec![1, 2, 3],
  };
  let wrapper = MessageWrapper::new(raw_producer);

  // Wrapper should be created
  assert!(true);
}

#[tokio::test]
async fn test_message_wrapper_with_id_generator() {
  let raw_producer = TestRawProducer {
    items: vec![1, 2, 3],
  };
  let id_generator = Arc::new(UuidGenerator::new());
  let wrapper = MessageWrapper::with_id_generator(raw_producer, id_generator);

  // Wrapper should be created
  assert!(true);
}

#[tokio::test]
async fn test_message_wrapper_inner() {
  let raw_producer = TestRawProducer {
    items: vec![1, 2, 3],
  };
  let wrapper = MessageWrapper::new(raw_producer);

  let inner = wrapper.inner();
  assert_eq!(inner.items, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_message_wrapper_inner_mut() {
  let raw_producer = TestRawProducer {
    items: vec![1, 2, 3],
  };
  let mut wrapper = MessageWrapper::new(raw_producer);

  let inner = wrapper.inner_mut();
  inner.items.push(4);
  assert_eq!(inner.items.len(), 4);
}

#[tokio::test]
async fn test_message_wrapper_into_inner() {
  let raw_producer = TestRawProducer {
    items: vec![1, 2, 3],
  };
  let wrapper = MessageWrapper::new(raw_producer);

  let inner = wrapper.into_inner();
  assert_eq!(inner.items, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_message_wrapper_produce() {
  let raw_producer = TestRawProducer {
    items: vec![1, 2, 3],
  };
  let mut wrapper = MessageWrapper::new(raw_producer);

  let mut stream = wrapper.produce();
  let mut results = Vec::new();
  while let Some(msg) = stream.next().await {
    results.push(msg.payload().clone());
  }

  assert_eq!(results, vec![1, 2, 3]);
}

struct TestRawConsumer {
  items: Vec<i32>,
}

#[async_trait::async_trait]
impl RawConsumer for TestRawConsumer {
  type Payload = i32;

  async fn consume_raw(
    &mut self,
    mut stream: std::pin::Pin<Box<dyn futures::Stream<Item = i32> + Send>>,
  ) {
    use futures::StreamExt;
    while let Some(item) = stream.next().await {
      self.items.push(item);
    }
  }
}

#[tokio::test]
async fn test_payload_extractor_consumer_new() {
  let raw_consumer = TestRawConsumer { items: Vec::new() };
  let consumer = PayloadExtractorConsumer::new(raw_consumer);

  // Consumer should be created
  assert!(true);
}

#[tokio::test]
async fn test_payload_extractor_consumer_inner() {
  let raw_consumer = TestRawConsumer { items: Vec::new() };
  let consumer = PayloadExtractorConsumer::new(raw_consumer);

  let inner = consumer.inner();
  assert_eq!(inner.items.len(), 0);
}

#[tokio::test]
async fn test_payload_extractor_consumer_inner_mut() {
  let raw_consumer = TestRawConsumer { items: Vec::new() };
  let mut consumer = PayloadExtractorConsumer::new(raw_consumer);

  let inner = consumer.inner_mut();
  inner.items.push(1);
  assert_eq!(inner.items.len(), 1);
}

#[tokio::test]
async fn test_payload_extractor_consumer_consume() {
  let raw_consumer = TestRawConsumer { items: Vec::new() };
  let mut consumer = PayloadExtractorConsumer::new(raw_consumer);

  let messages = vec![
    Message::new(1, MessageId::new_uuid()),
    Message::new(2, MessageId::new_uuid()),
  ];
  let input_stream = Box::pin(futures::stream::iter(messages));

  consumer.consume(input_stream).await;

  let inner = consumer.inner();
  assert_eq!(inner.items, vec![1, 2]);
}
