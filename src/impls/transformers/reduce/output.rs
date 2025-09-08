use crate::structs::transformers::reduce::ReduceTransformer;
use crate::traits::output::Output;
use futures::Stream;
use std::pin::Pin;

impl<T, Acc, F> Output for ReduceTransformer<T, Acc, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Acc: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(Acc, T) -> Acc + Send + Clone + 'static,
{
  type Output = Acc;
  type OutputStream = Pin<Box<dyn Stream<Item = Acc> + Send>>;
}
