use crate::structs::transformers::window::WindowTransformer;
use crate::traits::input::Input;
use futures::Stream;
use std::pin::Pin;

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Input for WindowTransformer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
