use crate::filter_transformer::FilterTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave_core::Input;

impl<F, T> Input for FilterTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
