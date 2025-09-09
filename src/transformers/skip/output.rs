use crate::output::Output;
use crate::transformers::skip::skip_transformer::SkipTransformer;
use futures::Stream;
use std::pin::Pin;

impl<T> Output for SkipTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
