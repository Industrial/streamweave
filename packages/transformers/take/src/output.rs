use crate::take_transformer::TakeTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for TakeTransformer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
