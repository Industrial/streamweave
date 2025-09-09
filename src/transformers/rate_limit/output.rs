use crate::output::Output;
use crate::transformers::rate_limit::rate_limit_transformer::RateLimitTransformer;
use futures::Stream;
use std::pin::Pin;

impl<T> Output for RateLimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
