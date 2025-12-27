use crate::csv_producer::CsvProducer;
use futures::Stream;
use serde::de::DeserializeOwned;
use std::pin::Pin;
use streamweave::Output;

impl<T> Output for CsvProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
