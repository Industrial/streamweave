use crate::time_window_transformer::TimeWindowTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Input;

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Input for TimeWindowTransformer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
