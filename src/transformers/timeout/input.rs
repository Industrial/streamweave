use crate::transformers::timeout::timeout_transformer::TimeoutTransformer;
use crate::input::Input;
use futures::Stream;
use std::pin::Pin;

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Input for TimeoutTransformer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
