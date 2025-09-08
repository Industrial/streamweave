use crate::structs::transformers::take::TakeTransformer;
use crate::traits::output::Output;
use futures::Stream;
use std::pin::Pin;

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for TakeTransformer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
