use crate::input::Input;
use crate::transformers::backpressure::backpressure_transformer::BackpressureTransformer;
use futures::Stream;
use std::pin::Pin;

impl<T> Input for BackpressureTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
