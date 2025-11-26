use crate::input::Input;
use crate::transformers::broadcast::broadcast_transformer::BroadcastTransformer;
use futures::Stream;
use std::pin::Pin;

impl<T> Input for BroadcastTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}
