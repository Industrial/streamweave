use crate::output::Output;
use crate::transformers::broadcast::broadcast_transformer::BroadcastTransformer;
use futures::Stream;
use std::pin::Pin;

impl<T> Output for BroadcastTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Output is a Vec of items, one copy for each consumer.
  /// Each output item contains all the clones for that input element.
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
