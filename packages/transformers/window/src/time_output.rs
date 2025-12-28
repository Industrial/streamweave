use crate::time_window_transformer::TimeWindowTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for TimeWindowTransformer<T> {
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}
