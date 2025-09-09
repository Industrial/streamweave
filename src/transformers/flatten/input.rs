use crate::transformers::flatten::flatten_transformer::FlattenTransformer;
use crate::input::Input;
use futures::Stream;
use std::pin::Pin;

impl<T> Input for FlattenTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = Vec<T>;
  type InputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}
