use crate::output::Output;
use crate::producers::kafka::kafka_producer::KafkaProducer;
use futures::Stream;
use std::pin::Pin;

impl Output for KafkaProducer {
  type Output = crate::producers::kafka::kafka_producer::KafkaMessage;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
