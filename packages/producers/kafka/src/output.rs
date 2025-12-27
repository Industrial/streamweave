use crate::kafka_producer::KafkaProducer;
use futures::Stream;
use std::pin::Pin;
use streamweave_core::Output;

impl Output for KafkaProducer {
  type Output = crate::kafka_producer::KafkaMessage;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
