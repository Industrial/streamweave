use crate::structs::transformers::map::MapTransformer;
use crate::traits::output::Output;
use futures::Stream;
use std::pin::Pin;

impl<F, I, O> Output for MapTransformer<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = O;
  type OutputStream = Pin<Box<dyn Stream<Item = O> + Send>>;
}
