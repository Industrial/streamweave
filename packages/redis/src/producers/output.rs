use super::redis_producer::RedisProducer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl Output for RedisProducer {
  type Output = super::redis_producer::RedisMessage;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
