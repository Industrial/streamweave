use crate::redis_streams_consumer::RedisStreamsConsumer;
use futures::Stream;
use serde::Serialize;
use std::pin::Pin;
use streamweave_core::Input;

impl<T> Input for RedisStreamsConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}
