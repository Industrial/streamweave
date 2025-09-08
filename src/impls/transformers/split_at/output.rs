use crate::structs::transformers::split_at::SplitAtTransformer;
use crate::traits::output::Output;
use futures::Stream;
use std::pin::Pin;

impl<T> Output for SplitAtTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = (Vec<T>, Vec<T>);
  type OutputStream = Pin<Box<dyn Stream<Item = (Vec<T>, Vec<T>)> + Send>>;
}
