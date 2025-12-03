//! # Path-Based Router Transformer
//!
//! Transformer that routes HTTP requests to different output ports based on path patterns.
//!
//! This transformer is designed for graph-based HTTP servers where requests need to be
//! routed to different handlers (REST, GraphQL, RPC, Static Files, etc.) based on their path.

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::http_server::types::HttpRequest;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::message::Message;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use crate::transformer::{Transformer, TransformerConfig};
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use async_trait::async_trait;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use futures::StreamExt;

/// Route pattern configuration for path-based routing.
#[derive(Debug, Clone)]
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub struct RoutePattern {
  /// The path pattern to match (supports wildcards like `/api/*`)
  pub pattern: String,
  /// The output port index for this route
  pub port: usize,
}

/// Configuration for path-based routing transformer.
#[derive(Debug, Clone, Default)]
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub struct PathRouterConfig {
  /// Route patterns to match against request paths
  pub routes: Vec<RoutePattern>,
  /// Default port for unmatched paths (None means drop unmatched requests)
  pub default_port: Option<usize>,
}

/// Transformer that routes HTTP requests to different output ports based on path patterns.
///
/// This transformer takes `Message<HttpRequest>` items and routes them to different
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
/// use streamweave::http_server::transformers::{PathBasedRouterTransformer, PathRouterConfig};
/// use streamweave::message::Message;
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
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub struct PathBasedRouterTransformer {
  /// Transformer configuration
  config: TransformerConfig<Message<HttpRequest>>,
  /// Path router configuration
  router_config: PathRouterConfig,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
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

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
#[async_trait]
impl Transformer for PathBasedRouterTransformer {
  type InputPorts = (Message<HttpRequest>,);
  // Output to 5 ports: REST (0), GraphQL (1), RPC (2), Static (3), Default (4)
  // Each port gets an Option<Message<HttpRequest>> - Some(item) if it matches, None otherwise
  type OutputPorts = (
    Option<Message<HttpRequest>>,
    Option<Message<HttpRequest>>,
    Option<Message<HttpRequest>>,
    Option<Message<HttpRequest>>,
    Option<Message<HttpRequest>>,
  );

  /// Transforms the input stream by routing requests to appropriate output ports.
  ///
  /// Each input item is matched against route patterns, and the item is placed
  /// in the appropriate output port tuple position. Other positions get `None`.
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
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

  fn set_config_impl(&mut self, config: TransformerConfig<Message<HttpRequest>>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Message<HttpRequest>> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Message<HttpRequest>> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Message<HttpRequest>>) -> ErrorAction {
    match &self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(
    &self,
    item: Option<Message<HttpRequest>>,
  ) -> ErrorContext<Message<HttpRequest>> {
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::message::MessageId;
  use futures::stream;
  use std::collections::HashMap;

  #[tokio::test]
  async fn test_path_matching() {
    let router = PathBasedRouterTransformer::new(PathRouterConfig::default());

    // Test exact match
    assert!(router.path_matches("/api/graphql", "/api/graphql"));
    assert!(!router.path_matches("/api/graphql", "/api/rest"));

    // Test wildcard match
    assert!(router.path_matches("/api/rest/*", "/api/rest/users"));
    assert!(router.path_matches("/api/rest/*", "/api/rest/users/123"));
    assert!(!router.path_matches("/api/rest/*", "/api/graphql"));
  }

  #[tokio::test]
  async fn test_route_matching() {
    // Create router with some routes
    let router = PathBasedRouterTransformer::new(PathRouterConfig {
      routes: vec![
        RoutePattern {
          pattern: "/api/rest/*".to_string(),
          port: 0,
        },
        RoutePattern {
          pattern: "/api/graphql".to_string(),
          port: 1,
        },
        RoutePattern {
          pattern: "/api/rpc/*".to_string(),
          port: 2,
        },
        RoutePattern {
          pattern: "/static/*".to_string(),
          port: 3,
        },
      ],
      default_port: Some(4),
    });

    assert_eq!(router.match_route("/api/rest/users"), Some(0));
    assert_eq!(router.match_route("/api/graphql"), Some(1));
    assert_eq!(router.match_route("/api/rpc/call"), Some(2));
    assert_eq!(router.match_route("/static/file.css"), Some(3));
    assert_eq!(router.match_route("/unknown/path"), Some(4)); // Default

    // Test empty router - should return None for unmatched paths
    let empty_router = PathBasedRouterTransformer::new(PathRouterConfig::default());
    assert_eq!(empty_router.match_route("/any/path"), None);
  }

  #[tokio::test]
  async fn test_routing_transformation() {
    let mut router = PathBasedRouterTransformer::new(PathRouterConfig {
      routes: vec![
        RoutePattern {
          pattern: "/api/rest/*".to_string(),
          port: 0,
        },
        RoutePattern {
          pattern: "/api/graphql".to_string(),
          port: 1,
        },
      ],
      default_port: None,
    });

    let request1 = HttpRequest {
      request_id: "req1".to_string(),
      method: crate::http_server::types::HttpMethod::Get,
      uri: axum::http::Uri::from_static("/api/rest/users"),
      path: "/api/rest/users".to_string(),
      headers: axum::http::HeaderMap::new(),
      query_params: HashMap::new(),
      path_params: HashMap::new(),
      body: None,
      content_type: None,
      remote_addr: None,
    };

    let request2 = HttpRequest {
      request_id: "req2".to_string(),
      method: crate::http_server::types::HttpMethod::Post,
      uri: axum::http::Uri::from_static("/api/graphql"),
      path: "/api/graphql".to_string(),
      headers: axum::http::HeaderMap::new(),
      query_params: HashMap::new(),
      path_params: HashMap::new(),
      body: None,
      content_type: None,
      remote_addr: None,
    };

    let msg1 = Message::new(request1, MessageId::new_custom("msg1"));
    let msg2 = Message::new(request2, MessageId::new_custom("msg2"));

    let input = stream::iter(vec![msg1, msg2]);
    let boxed_input = Box::pin(input);

    let results: Vec<_> = router.transform(boxed_input).collect().await;

    // First request should go to port 0 (REST)
    assert!(results[0].0.is_some());
    assert!(results[0].1.is_none());
    assert!(results[0].2.is_none());
    assert!(results[0].3.is_none());
    assert!(results[0].4.is_none());

    // Second request should go to port 1 (GraphQL)
    assert!(results[1].0.is_none());
    assert!(results[1].1.is_some());
    assert!(results[1].2.is_none());
    assert!(results[1].3.is_none());
    assert!(results[1].4.is_none());
  }
}
