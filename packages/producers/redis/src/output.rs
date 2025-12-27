use crate::redis_producer::RedisProducer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl Output for RedisProducer {
  type Output = crate::redis_producer::RedisMessage;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
