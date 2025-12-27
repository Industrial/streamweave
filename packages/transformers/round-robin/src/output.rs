use crate::round_robin_transformer::RoundRobinTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl<T> Output for RoundRobinTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Output is a tuple of (consumer_index, element).
  /// This allows downstream processing to route elements to the correct consumer.
  type Output = (usize, T);
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
