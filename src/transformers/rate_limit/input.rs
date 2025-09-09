use crate::transformers::rate_limit::rate_limit_transformer::RateLimitTransformer;
use crate::input::Input;
use futures::Stream;
use std::pin::Pin;

impl<T> Input for RateLimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
