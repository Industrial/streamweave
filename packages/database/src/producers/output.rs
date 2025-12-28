use super::database_producer::DatabaseProducer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl Output for DatabaseProducer {
  type Output = super::database_producer::DatabaseRow;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
