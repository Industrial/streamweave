use crate::structs::transformers::partition::PartitionTransformer;
use crate::traits::output::Output;
use futures::Stream;
use std::pin::Pin;

impl<F, T> Output for PartitionTransformer<F, T>
where
  F: Fn(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = (Vec<T>, Vec<T>);
  type OutputStream = Pin<Box<dyn Stream<Item = (Vec<T>, Vec<T>)> + Send>>;
}
