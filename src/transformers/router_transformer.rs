//! # Router Transformer
//!
//! Transformer that routes elements based on content to specific destinations.
//! This enables conditional routing, fan-out patterns, and dynamic destination
//! selection based on item content or routing logic.
//!
//! ## Overview
//!
//! The Router Transformer provides:
//!
//! - **Content-Based Routing**: Routes items based on routing function decisions
//! - **Multiple Destinations**: Supports named destinations, indices, default, and drop
//! - **Dynamic Routing**: Routing decisions made per item based on content
//! - **Type Generic**: Works with any `Send + Sync + Clone` type
//! - **Error Handling**: Configurable error strategies
//!
//! ## Input/Output
//!
//! - **Input**: `Message<T>` - Items to route
//! - **Output**: `Message<T>` - Routed items (may be filtered/dropped based on routing)
//!
//! ## Route Targets
//!
//! - **Named**: Route to a specific named destination
//! - **Index**: Route to a numeric index destination
//! - **Default**: Route to the default destination
//! - **Drop**: Drop the item (don't route)
//!
//! ## Example
//!
//! ```rust
//! use crate::transformers::{RouterTransformer, RouteTarget};
//!
//! let transformer = RouterTransformer::new(|item: &i32| {
//!     if *item > 10 {
//!         RouteTarget::named("high")
//!     } else {
//!         RouteTarget::named("low")
//!     }
//! });
//! ```

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::marker::PhantomData;
use std::pin::Pin;

/// Represents the result of a routing decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteTarget {
  /// Route to a specific named destination.
  Named(String),
  /// Route to a numeric index.
  Index(usize),
  /// Route to the default destination.
  Default,
  /// Drop the element (don't route).
  Drop,
}

impl RouteTarget {
  /// Creates a new named route target.
  #[must_use]
  pub fn named(name: impl Into<String>) -> Self {
    Self::Named(name.into())
  }

  /// Creates a new index route target.
  #[must_use]
  pub fn index(idx: usize) -> Self {
    Self::Index(idx)
  }

  /// Creates a default route target.
  #[must_use]
  pub fn default_route() -> Self {
    Self::Default
  }

  /// Creates a drop route target.
  #[must_use]
  pub fn drop() -> Self {
    Self::Drop
  }
}

/// A transformer that routes elements based on content to specific consumers.
///
/// This implements content-based routing where a routing function determines
/// which consumer should receive each element. This is useful for:
/// - Directing events to different handlers based on type
/// - Partitioning data by key
/// - Filtering to different outputs
pub struct RouterTransformer<T, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: Fn(&T) -> RouteTarget + Clone + Send + Sync + 'static,
{
  /// Configuration for the transformer.
  pub config: TransformerConfig<T>,
  /// The routing function.
  pub router_fn: F,
  /// Default target when router_fn returns Default.
  pub default_target: Option<RouteTarget>,
  /// Phantom data for the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T, F> Clone for RouterTransformer<T, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: Fn(&T) -> RouteTarget + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      router_fn: self.router_fn.clone(),
      default_target: self.default_target.clone(),
      _phantom: PhantomData,
    }
  }
}

impl<T, F> RouterTransformer<T, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: Fn(&T) -> RouteTarget + Clone + Send + Sync + 'static,
{
  /// Creates a new RouterTransformer with the specified routing function.
  #[must_use]
  pub fn new(router_fn: F) -> Self {
    Self {
      config: TransformerConfig::default(),
      router_fn,
      default_target: None,
      _phantom: PhantomData,
    }
  }

  /// Sets the default target for unrouted elements.
  #[must_use]
  pub fn with_default_target(mut self, target: RouteTarget) -> Self {
    self.default_target = Some(target);
    self
  }

  /// Sets the error strategy for the transformer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config = self.config.with_error_strategy(strategy);
    self
  }

  /// Sets the name for the transformer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config = self.config.with_name(name);
    self
  }

  /// Routes an element and returns the target.
  pub fn route(&self, item: &T) -> RouteTarget {
    let target = (self.router_fn)(item);
    match target {
      RouteTarget::Default => self.default_target.clone().unwrap_or(RouteTarget::Default),
      other => other,
    }
  }
}

impl<T, F> Input for RouterTransformer<T, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: Fn(&T) -> RouteTarget + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

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

#[async_trait]
impl<T, F> Transformer for RouterTransformer<T, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: Fn(&T) -> RouteTarget + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = ((RouteTarget, T),);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let router_fn = self.router_fn.clone();
    let default_target = self.default_target.clone();

    Box::pin(input.filter_map(move |item| {
      let target = router_fn(&item);
      let resolved_target = match target {
        RouteTarget::Default => default_target.clone().unwrap_or(RouteTarget::Default),
        other => other,
      };

      async move {
        match resolved_target {
          RouteTarget::Drop => None, // Filter out dropped elements
          target => Some((target, item)),
        }
      }
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match &self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      ErrorStrategy::Custom(handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "router_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "router_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
