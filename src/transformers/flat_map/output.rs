use crate::output::Output;
use crate::transformers::flat_map::flat_map_transformer::FlatMapTransformer;
use futures::Stream;
use std::pin::Pin;

impl<F, I, O> Output for FlatMapTransformer<F, I, O>
where
  F: Fn(I) -> Vec<O> + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = O;
  type OutputStream = Pin<Box<dyn Stream<Item = O> + Send>>;
}
