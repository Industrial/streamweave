use crate::structs::transformers::take::TakeTransformer;
use crate::traits::input::Input;
use futures::Stream;
use std::pin::Pin;

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Input for TakeTransformer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
