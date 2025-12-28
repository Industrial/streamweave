use super::circuit_breaker_transformer::CircuitBreakerTransformer;
use futures::Stream;
use std::pin::Pin;
use streamweave::Input;

impl<T> Input for CircuitBreakerTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
