use crate::structs::transformers::timeout::TimeoutTransformer;
use crate::traits::input::Input;
use futures::Stream;
use std::pin::Pin;

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Input for TimeoutTransformer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
