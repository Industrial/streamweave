use crate::round_robin_transformer::RoundRobinTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave_core::Input;

impl<T> Input for RoundRobinTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}
