use crate::split_transformer::SplitTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave_core::Input;

impl<F, T> Input for SplitTransformer<F, T>
where
  F: Send + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = Vec<T>;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}
