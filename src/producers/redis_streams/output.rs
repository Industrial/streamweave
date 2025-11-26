use crate::output::Output;
use crate::producers::redis_streams::redis_streams_producer::RedisStreamsProducer;
use futures::Stream;
use std::pin::Pin;

impl Output for RedisStreamsProducer {
  type Output = crate::producers::redis_streams::redis_streams_producer::RedisStreamsMessage;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
