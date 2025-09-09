use crate::output::Output;
use crate::transformers::timeout::timeout_transformer::TimeoutTransformer;
use futures::Stream;
use std::pin::Pin;

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for TimeoutTransformer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
