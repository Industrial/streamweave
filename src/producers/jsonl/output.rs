use crate::output::Output;
use crate::producers::jsonl::jsonl_producer::JsonlProducer;
use futures::Stream;
use serde::de::DeserializeOwned;
use std::pin::Pin;

impl<T> Output for JsonlProducer<T>
where
  T: DeserializeOwned + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
