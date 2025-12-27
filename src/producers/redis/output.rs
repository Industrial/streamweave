use crate::output::Output;
use crate::producers::redis::redis_producer::RedisProducer;
use futures::Stream;
use std::pin::Pin;

impl Output for RedisProducer {
  type Output = crate::producers::redis::redis_producer::RedisMessage;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
