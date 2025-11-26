use crate::input::Input;
use crate::transformers::router::router_transformer::{RouteTarget, RouterTransformer};
use futures::Stream;
use std::pin::Pin;

impl<T, F> Input for RouterTransformer<T, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: Fn(&T) -> RouteTarget + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}
