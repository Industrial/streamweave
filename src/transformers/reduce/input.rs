use crate::transformers::reduce::reduce_transformer::ReduceTransformer;
use crate::input::Input;
use futures::Stream;
use std::pin::Pin;

impl<T, Acc, F> Input for ReduceTransformer<T, Acc, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Acc: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(Acc, T) -> Acc + Send + Clone + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
