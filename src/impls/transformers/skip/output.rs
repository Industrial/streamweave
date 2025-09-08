use crate::structs::transformers::skip::SkipTransformer;
use crate::traits::output::Output;
use futures::Stream;
use std::pin::Pin;

impl<T> Output for SkipTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
