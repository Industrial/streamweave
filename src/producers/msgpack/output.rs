use crate::output::Output;
use crate::producers::msgpack::msgpack_producer::MsgPackProducer;
use futures::Stream;
use serde::de::DeserializeOwned;
use std::pin::Pin;

impl<T> Output for MsgPackProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
