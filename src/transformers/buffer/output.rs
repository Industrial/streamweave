use crate::output::Output;
use crate::transformers::buffer::buffer_transformer::BufferTransformer;
use futures::Stream;
use std::pin::Pin;

impl<T> Output for BufferTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
