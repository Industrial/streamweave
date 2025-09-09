use crate::input::Input;
use crate::transformers::limit::limit_transformer::LimitTransformer;
use futures::Stream;
use std::pin::Pin;

impl<T> Input for LimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
