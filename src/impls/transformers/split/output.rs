use crate::structs::transformers::split::SplitTransformer;
use crate::traits::output::Output;
use futures::Stream;
use std::pin::Pin;

impl<F, T> Output for SplitTransformer<F, T>
where
  F: Send + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
