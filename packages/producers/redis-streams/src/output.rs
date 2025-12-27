use crate::redis_streams_producer::RedisStreamsProducer;
use futures::Stream;
use std::pin::Pin;
use streamweave_core::Output;

impl Output for RedisStreamsProducer {
  type Output = crate::redis_streams_producer::RedisStreamsMessage;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
