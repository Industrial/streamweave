use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use std::marker::PhantomData;

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
///
/// # Example
///
/// ```ignore
/// use streamweave::transformers::router::router_transformer::{RouterTransformer, RouteTarget};
///
/// let router = RouterTransformer::<i32, _>::new(|x: &i32| {
///     if *x % 2 == 0 {
///         RouteTarget::named("even")
///     } else {
///         RouteTarget::named("odd")
///     }
/// });
/// ```
#[derive(Clone)]
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
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the transformer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_route_target_named() {
    let target = RouteTarget::named("test");
    assert_eq!(target, RouteTarget::Named("test".to_string()));
  }

  #[test]
  fn test_route_target_index() {
    let target = RouteTarget::index(5);
    assert_eq!(target, RouteTarget::Index(5));
  }

  #[test]
  fn test_route_target_default() {
    let target = RouteTarget::default_route();
    assert_eq!(target, RouteTarget::Default);
  }

  #[test]
  fn test_route_target_drop() {
    let target = RouteTarget::drop();
    assert_eq!(target, RouteTarget::Drop);
  }

  #[test]
  fn test_router_transformer_new() {
    let router = RouterTransformer::<i32, _>::new(|x: &i32| {
      if *x % 2 == 0 {
        RouteTarget::named("even")
      } else {
        RouteTarget::named("odd")
      }
    });

    assert_eq!(router.route(&2), RouteTarget::Named("even".to_string()));
    assert_eq!(router.route(&3), RouteTarget::Named("odd".to_string()));
  }

  #[test]
  fn test_router_transformer_with_default() {
    let router = RouterTransformer::<i32, _>::new(|x: &i32| {
      if *x > 10 {
        RouteTarget::named("high")
      } else {
        RouteTarget::Default
      }
    })
    .with_default_target(RouteTarget::named("low"));

    assert_eq!(router.route(&15), RouteTarget::Named("high".to_string()));
    assert_eq!(router.route(&5), RouteTarget::Named("low".to_string()));
  }

  #[test]
  fn test_router_transformer_clone() {
    let router = RouterTransformer::<i32, _>::new(|_: &i32| RouteTarget::named("test"))
      .with_name("original".to_string());

    let cloned = router.clone();
    assert_eq!(cloned.config.name, Some("original".to_string()));
    assert_eq!(cloned.route(&1), RouteTarget::Named("test".to_string()));
  }

  #[test]
  fn test_router_with_index_routing() {
    let router = RouterTransformer::<i32, _>::new(|x: &i32| RouteTarget::index((*x as usize) % 4));

    assert_eq!(router.route(&0), RouteTarget::Index(0));
    assert_eq!(router.route(&1), RouteTarget::Index(1));
    assert_eq!(router.route(&4), RouteTarget::Index(0));
    assert_eq!(router.route(&7), RouteTarget::Index(3));
  }

  #[test]
  fn test_router_with_drop() {
    let router = RouterTransformer::<i32, _>::new(|x: &i32| {
      if *x < 0 {
        RouteTarget::Drop
      } else {
        RouteTarget::named("positive")
      }
    });

    assert_eq!(router.route(&-5), RouteTarget::Drop);
    assert_eq!(router.route(&5), RouteTarget::Named("positive".to_string()));
  }

  #[test]
  fn test_router_with_string_content() {
    let router = RouterTransformer::<String, _>::new(|s: &String| {
      if s.starts_with("ERROR") {
        RouteTarget::named("errors")
      } else if s.starts_with("WARN") {
        RouteTarget::named("warnings")
      } else {
        RouteTarget::named("info")
      }
    });

    assert_eq!(
      router.route(&"ERROR: Something failed".to_string()),
      RouteTarget::Named("errors".to_string())
    );
    assert_eq!(
      router.route(&"WARN: Low memory".to_string()),
      RouteTarget::Named("warnings".to_string())
    );
    assert_eq!(
      router.route(&"INFO: Starting".to_string()),
      RouteTarget::Named("info".to_string())
    );
  }
}
