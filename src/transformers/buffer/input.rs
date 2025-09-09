use crate::transformers::buffer::buffer_transformer::BufferTransformer;
use crate::input::Input;
use futures::Stream;
use std::pin::Pin;

impl<T> Input for BufferTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
