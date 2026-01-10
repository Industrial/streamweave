//! Router node for StreamWeave graphs
//!
//! Generic routing node that routes items based on a routing function. Routes
//! elements based on content to specific consumers.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{RouteTarget, RouterTransformer};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Generic routing node that routes items based on a routing function.
///
/// This node wraps `RouterTransformer` for use in graphs. It routes elements
/// based on content to specific consumers using a routing function.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{Router, TransformerNode};
/// use crate::transformers::RouteTarget;
///
/// let router = Router::new(|x: &i32| {
///     if *x > 10 {
///         RouteTarget::named("high")
///     } else {
///         RouteTarget::named("low")
///     }
/// });
/// let node = TransformerNode::from_transformer(
///     "router".to_string(),
///     router,
/// );
/// ```
pub struct Router<T, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: Fn(&T) -> RouteTarget + Clone + Send + Sync + 'static,
{
  /// The underlying router transformer
  transformer: RouterTransformer<T, F>,
}

impl<T, F> Router<T, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: Fn(&T) -> RouteTarget + Clone + Send + Sync + 'static,
{
  /// Creates a new `Router` node with the specified routing function.
  ///
  /// # Arguments
  ///
  /// * `router_fn` - The routing function that determines where each item should be routed.
  pub fn new(router_fn: F) -> Self {
    Self {
      transformer: RouterTransformer::new(router_fn),
    }
  }

  /// Sets the error handling strategy for this node.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }
}

impl<T, F> Clone for Router<T, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: Fn(&T) -> RouteTarget + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T, F> Input for Router<T, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: Fn(&T) -> RouteTarget + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T, F> Output for Router<T, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: Fn(&T) -> RouteTarget + Clone + Send + Sync + 'static,
{
  type Output = (RouteTarget, T);
  type OutputStream = Pin<Box<dyn Stream<Item = (RouteTarget, T)> + Send>>;
}

#[async_trait]
impl<T, F> Transformer for Router<T, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: Fn(&T) -> RouteTarget + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = ((RouteTarget, T),);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}
