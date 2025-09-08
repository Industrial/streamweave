use crate::structs::transformers::sample::SampleTransformer;
use crate::traits::input::Input;
use futures::Stream;
use std::pin::Pin;

impl<T> Input for SampleTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
