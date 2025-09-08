use crate::structs::transformers::flat_map::FlatMapTransformer;
use crate::traits::input::Input;
use futures::Stream;
use std::pin::Pin;

impl<F, I, O> Input for FlatMapTransformer<F, I, O>
where
  F: Fn(I) -> Vec<O> + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = I;
  type InputStream = Pin<Box<dyn Stream<Item = I> + Send>>;
}
