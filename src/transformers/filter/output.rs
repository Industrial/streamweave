use crate::output::Output;
use crate::transformers::filter::filter_transformer::FilterTransformer;
use futures::Stream;
use std::pin::Pin;

impl<F, T> Output for FilterTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
