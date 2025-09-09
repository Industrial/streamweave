use crate::input::Input;
use crate::transformers::delay::delay_transformer::DelayTransformer;
use futures::Stream;
use std::pin::Pin;

impl<T> Input for DelayTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
