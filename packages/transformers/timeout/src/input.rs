use crate::timeout_transformer::TimeoutTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave_core::Input;

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Input for TimeoutTransformer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
