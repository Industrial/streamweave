//! Router node for routing items to different destinations based on content.
//!
//! This module provides [`Router`], a graph node that routes items based on a
//! routing function. It routes elements based on content to specific consumers
//! using a routing function that determines the destination for each item. It
//! wraps [`RouterTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`Router`] is useful for routing items to different destinations in graph-based
//! pipelines. It uses a routing function to determine where each item should be
//! sent, enabling conditional routing, load balancing, and content-based
//! distribution patterns.
//!
//! # Key Concepts
//!
//! - **Content-Based Routing**: Routes items based on their content using a function
//! - **Route Targets**: Uses `RouteTarget` to specify destination for each item
//! - **Flexible Routing**: Supports named routes, indices, and custom routing logic
//! - **Transformer Wrapper**: Wraps `RouterTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`Router<T, F>`]**: Node that routes items based on a routing function
//! - **[`RouteTarget`]**: Enum representing routing destinations (named, indexed, etc.)
//!
//! # Quick Start
//!
//! ## Basic Usage (Named Routes)
//!
//! ```rust
//! use streamweave::graph::nodes::Router;
//! use streamweave::transformers::RouteTarget;
//!
//! // Route items to "high" or "low" based on value
//! let router = Router::new(|x: &i32| {
//!     if *x > 10 {
//!         RouteTarget::named("high")
//!     } else {
//!         RouteTarget::named("low")
//!     }
//! });
//! ```
//!
//! ## Index-Based Routing
//!
//! ```rust
//! use streamweave::graph::nodes::Router;
//! use streamweave::transformers::RouteTarget;
//!
//! // Route items to different indices based on modulo
//! let router = Router::new(|x: &i32| {
//!     RouteTarget::index((*x % 3) as usize)
//! });
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::Router;
//! use streamweave::transformers::RouteTarget;
//! use streamweave::ErrorStrategy;
//!
//! // Create a router with error handling
//! let router = Router::new(|x: &i32| RouteTarget::named("default"))
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("value-router".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Routing Function**: Uses a closure for flexible routing logic
//! - **RouteTarget Enum**: Uses enum-based routing targets for type safety
//! - **Content-Based**: Routes based on item content for dynamic routing
//! - **Output Type**: Outputs `(RouteTarget, T)` tuples for routing information
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`Router`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`]. Note that the
//! output type is `(RouteTarget, T)` rather than `T`.

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
