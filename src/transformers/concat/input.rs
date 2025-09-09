use crate::input::Input;
use crate::transformers::concat::concat_transformer::ConcatTransformer;
use futures::Stream;
use std::pin::Pin;

impl<T> Input for ConcatTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
