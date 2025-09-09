use crate::transformers::concat::concat_transformer::ConcatTransformer;
use crate::output::Output;
use futures::Stream;
use std::pin::Pin;

impl<T> Output for ConcatTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
