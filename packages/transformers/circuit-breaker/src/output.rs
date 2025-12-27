use crate::circuit_breaker_transformer::CircuitBreakerTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Output;

impl<T> Output for CircuitBreakerTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
