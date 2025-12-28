use super::database_consumer::DatabaseConsumer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Input;

impl Input for DatabaseConsumer {
  type Input = super::super::producers::database_producer::DatabaseRow;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}
