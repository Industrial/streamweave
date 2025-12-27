use crate::rate_limit_transformer::RateLimitTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave_core::Input;

impl<T> Input for RateLimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
