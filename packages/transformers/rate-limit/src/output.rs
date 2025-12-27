use crate::rate_limit_transformer::RateLimitTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave_core::Output;

impl<T> Output for RateLimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
