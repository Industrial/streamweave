use crate::timeout_transformer::TimeoutTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave_core::Output;

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for TimeoutTransformer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
