use crate::transformers::circuit_breaker::circuit_breaker_transformer::CircuitBreakerTransformer;
use crate::output::Output;
use futures::Stream;
use std::pin::Pin;

impl<T> Output for CircuitBreakerTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}
