use crate::input::Input;
use crate::transformers::debounce::debounce_transformer::DebounceTransformer;
use futures::Stream;
use std::pin::Pin;

impl<T> Input for DebounceTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
