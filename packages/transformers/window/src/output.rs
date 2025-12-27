use crate::window_transformer::WindowTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave_core::Output;

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for WindowTransformer<T> {
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}
