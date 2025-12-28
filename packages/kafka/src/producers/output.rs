use super::kafka_producer::KafkaProducer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl Output for KafkaProducer {
  type Output = super::kafka_producer::KafkaMessage;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
