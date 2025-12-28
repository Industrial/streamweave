use super::kafka_consumer::KafkaConsumer;
use futures::Stream;
use serde::Serialize;
use std::pin::Pin;
use streamweave::Input;

impl<T> Input for KafkaConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}
