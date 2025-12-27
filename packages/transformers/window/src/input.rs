use crate::window_transformer::WindowTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave_core::Input;

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Input for WindowTransformer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
