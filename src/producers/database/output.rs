use crate::output::Output;
use crate::producers::database::database_producer::DatabaseProducer;
use futures::Stream;
use std::pin::Pin;

impl Output for DatabaseProducer {
  type Output = crate::producers::database::database_producer::DatabaseRow;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
