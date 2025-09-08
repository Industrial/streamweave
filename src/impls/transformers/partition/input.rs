use crate::structs::transformers::partition::PartitionTransformer;
use crate::traits::input::Input;
use futures::Stream;
use std::pin::Pin;

impl<F, T> Input for PartitionTransformer<F, T>
where
  F: Fn(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
