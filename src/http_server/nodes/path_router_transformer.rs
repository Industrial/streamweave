//! # Path-Based Router Transformer
//!
//! Transformer that routes HTTP requests to different output ports based on path patterns.
//!
//! This transformer is designed for graph-based HTTP servers where requests need to be
//! routed to different handlers (REST, GraphQL, RPC, Static Files, etc.) based on their path.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::http_server::types::HttpServerRequest;
use crate::message::Message;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use std::pin::Pin;

/// Route pattern configuration for path-based routing.
#[derive(Debug, Clone)]
pub struct RoutePattern {
  /// The path pattern to match (supports wildcards like `/api/*`)
  pub pattern: String,
  /// The output port index for this route
  pub port: usize,
}

/// Configuration for path-based routing transformer.
#[derive(Debug, Clone, Default)]
pub struct PathRouterConfig {
  /// Route patterns to match against request paths
  pub routes: Vec<RoutePattern>,
  /// Default port for unmatched paths (None means drop unmatched requests)
  pub default_port: Option<usize>,
}

/// Transformer that routes HTTP requests to different output ports based on path patterns.
///
/// This transformer takes `Message<HttpServerRequest>` items and routes them to different
/// output ports based on matching path patterns. It supports wildcard patterns and
/// configurable route mappings.
///
/// The router starts empty by default - you must add routes explicitly using
/// `add_route()` or by providing a `PathRouterConfig` with routes. If no route
/// matches and no default port is configured, unmatched requests are dropped.
///
/// ## Example
///
/// ```rust,no_run
/// use crate::http_server::transformers::{PathBasedRouterTransformer, PathRouterConfig};
/// use crate::message::Message;
///
/// // Create an empty router
/// let mut router = PathBasedRouterTransformer::new(PathRouterConfig::default());
///
/// // Add routes explicitly
/// router.add_route("/api/rest/*".to_string(), 0);
/// router.add_route("/api/graphql".to_string(), 1);
/// router.set_default_port(Some(4)); // Route unmatched requests to port 4
/// ```
#[derive(Debug, Clone)]
pub struct PathBasedRouterTransformer {
  /// Transformer configuration
  config: TransformerConfig<Message<HttpServerRequest>>,
  /// Path router configuration
  router_config: PathRouterConfig,
}

impl PathBasedRouterTransformer {
  /// Creates a new path-based router transformer with the given configuration.
  ///
  /// The router starts empty by default - you must add routes explicitly.
  /// If no routes match and `default_port` is `None`, the request is dropped.
  pub fn new(router_config: PathRouterConfig) -> Self {
    Self {
      config: TransformerConfig::default(),
      router_config,
    }
  }

  /// Creates a new path-based router transformer with custom route patterns.
  ///
  /// # Arguments
  ///
  /// * `routes` - Route patterns to match against request paths
  /// * `default_port` - Port for unmatched paths (None means drop unmatched requests)
  pub fn with_routes(routes: Vec<RoutePattern>, default_port: Option<usize>) -> Self {
    Self::new(PathRouterConfig {
      routes,
      default_port,
    })
  }

  /// Adds a route pattern to the router.
  ///
  /// # Arguments
  ///
  /// * `pattern` - The path pattern to match (supports wildcards like `/api/*`)
  /// * `port` - The output port index for this route
  pub fn add_route(&mut self, pattern: String, port: usize) {
    self
      .router_config
      .routes
      .push(RoutePattern { pattern, port });
  }

  /// Sets the default port for unmatched requests.
  ///
  /// # Arguments
  ///
  /// * `port` - The port index for unmatched requests, or `None` to drop them
  pub fn set_default_port(&mut self, port: Option<usize>) {
    self.router_config.default_port = port;
  }

  /// Returns a reference to the router configuration.
  #[must_use]
  pub fn router_config(&self) -> &PathRouterConfig {
    &self.router_config
  }

  /// Returns a mutable reference to the router configuration.
  pub fn router_config_mut(&mut self) -> &mut PathRouterConfig {
    &mut self.router_config
  }

  /// Matches a path against route patterns and returns the port index.
  ///
  /// # Arguments
  ///
  /// * `path` - The request path to match
  ///
  /// # Returns
  ///
  /// The port index for the matched route, or the default port if no match.
  /// Returns `None` if no route matches and no default port is configured.
  fn match_route(&self, path: &str) -> Option<usize> {
    for route in &self.router_config.routes {
      if self.path_matches(&route.pattern, path) {
        return Some(route.port);
      }
    }
    self.router_config.default_port
  }

  /// Checks if a path matches a pattern (supports wildcards).
  ///
  /// # Arguments
  ///
  /// * `pattern` - The pattern to match against (supports `*` wildcard)
  /// * `path` - The path to check
  ///
  /// # Returns
  ///
  /// `true` if the path matches the pattern, `false` otherwise.
  fn path_matches(&self, pattern: &str, path: &str) -> bool {
    // Simple wildcard matching: `*` matches any sequence of characters
    if let Some(prefix) = pattern.strip_suffix("/*") {
      path.starts_with(prefix)
    } else {
      pattern == path
    }
  }
}

impl Input for PathBasedRouterTransformer {
  type Input = Message<HttpServerRequest>;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

impl Output for PathBasedRouterTransformer {
  type Output = (
    Option<Message<HttpServerRequest>>,
    Option<Message<HttpServerRequest>>,
    Option<Message<HttpServerRequest>>,
    Option<Message<HttpServerRequest>>,
    Option<Message<HttpServerRequest>>,
  );
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl Transformer for PathBasedRouterTransformer {
  type InputPorts = (Message<HttpServerRequest>,);
  // Output to 5 ports: REST (0), GraphQL (1), RPC (2), Static (3), Default (4)
  // Each port gets an Option<Message<HttpServerRequest>> - Some(item) if it matches, None otherwise
  type OutputPorts = (
    Option<Message<HttpServerRequest>>,
    Option<Message<HttpServerRequest>>,
    Option<Message<HttpServerRequest>>,
    Option<Message<HttpServerRequest>>,
    Option<Message<HttpServerRequest>>,
  );

  /// Transforms the input stream by routing requests to appropriate output ports.
  ///
  /// Each input item is matched against route patterns, and the item is placed
  /// in the appropriate output port tuple position. Other positions get `None`.
  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let router_config = self.router_config.clone();

    Box::pin(input.map(move |item| {
      let path = item.payload().path.clone();
      let port = Self {
        config: TransformerConfig::default(),
        router_config: router_config.clone(),
      }
      .match_route(&path);

      // Create output tuple with item in the matched port, None in others
      // If port is None, drop the request (all ports get None)
      match port {
        Some(0) => (Some(item.clone()), None, None, None, None),
        Some(1) => (None, Some(item.clone()), None, None, None),
        Some(2) => (None, None, Some(item.clone()), None, None),
        Some(3) => (None, None, None, Some(item.clone()), None),
        Some(4) => (None, None, None, None, Some(item.clone())),
        Some(_) => (None, None, None, None, Some(item)), // Default to port 4 for any other port
        None => (None, None, None, None, None), // Drop unmatched requests when no default port
      }
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Message<HttpServerRequest>>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Message<HttpServerRequest>> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Message<HttpServerRequest>> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Message<HttpServerRequest>>) -> ErrorAction {
    match &self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(
    &self,
    item: Option<Message<HttpServerRequest>>,
  ) -> ErrorContext<Message<HttpServerRequest>> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "path_router".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
