use crate::router_transformer::{RouteTarget, RouterTransformer};
use futures::Stream;
use std::pin::Pin;
use streamweave_core::Output;

impl<T, F> Output for RouterTransformer<T, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: Fn(&T) -> RouteTarget + Clone + Send + Sync + 'static,
{
  /// Output is a tuple of (route_target, element).
  /// Elements with RouteTarget::Drop are filtered out.
  type Output = (RouteTarget, T);
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
