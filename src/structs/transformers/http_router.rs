use crate::structs::http::{
  http_handler::HttpHandler, http_middleware::HttpMiddleware,
  http_request_chunk::StreamWeaveHttpRequestChunk, route_pattern::RoutePattern,
};
use crate::traits::transformer::TransformerConfig;
use std::collections::HashMap;
use std::sync::Arc;

/// HTTP Router Transformer configuration
#[derive(Clone)]
pub struct HttpRouterTransformer {
  pub routes: HashMap<RoutePattern, String>, // Pattern -> Handler ID
  pub handlers: HashMap<String, Arc<dyn HttpHandler>>, // Handler ID -> Handler
  pub middleware: HashMap<String, Arc<dyn HttpMiddleware>>, // Middleware ID -> Middleware
  pub fallback_handler: Option<Arc<dyn HttpHandler>>,
  pub config: TransformerConfig<StreamWeaveHttpRequestChunk>,
}

impl HttpRouterTransformer {
  /// Create a new HTTP Router Transformer
  pub fn new() -> Self {
    Self {
      routes: HashMap::new(),
      handlers: HashMap::new(),
      middleware: HashMap::new(),
      fallback_handler: None,
      config: TransformerConfig::default(),
    }
  }

  /// Add a route to the router
  pub fn add_route(
    mut self,
    pattern: RoutePattern,
    handler_id: String,
    handler: Arc<dyn HttpHandler>,
  ) -> Self {
    self.routes.insert(pattern, handler_id.clone());
    self.handlers.insert(handler_id, handler);
    self
  }

  /// Add middleware to the router
  pub fn add_middleware(
    mut self,
    middleware_id: String,
    middleware: Arc<dyn HttpMiddleware>,
  ) -> Self {
    self.middleware.insert(middleware_id, middleware);
    self
  }

  /// Set the fallback handler for unmatched routes
  pub fn with_fallback_handler(mut self, handler: Arc<dyn HttpHandler>) -> Self {
    self.fallback_handler = Some(handler);
    self
  }

  /// Set configuration
  pub fn with_config(mut self, config: TransformerConfig<StreamWeaveHttpRequestChunk>) -> Self {
    self.config = config;
    self
  }

  /// Find the best matching route for a request
  pub fn find_route(
    &self,
    method: &http::Method,
    path: &str,
  ) -> Option<(String, HashMap<String, String>)> {
    let mut best_match: Option<(String, HashMap<String, String>, u32)> = None;

    for (pattern, handler_id) in &self.routes {
      if let Some(params) = pattern.matches(method, path) {
        let priority = pattern.priority;

        match best_match {
          None => {
            best_match = Some((handler_id.clone(), params, priority));
          }
          Some((_, _, best_priority)) if priority > best_priority => {
            best_match = Some((handler_id.clone(), params, priority));
          }
          _ => {} // Current match is not better
        }
      }
    }

    best_match.map(|(handler_id, params, _)| (handler_id, params))
  }
}

impl Default for HttpRouterTransformer {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::structs::http::http_response::StreamWeaveHttpResponse;
  use http::Method;
  use std::sync::Arc;

  // Test handler implementation
  struct TestHandler {
    response: StreamWeaveHttpResponse,
  }

  #[async_trait::async_trait]
  impl HttpHandler for TestHandler {
    async fn handle(&self, _request: StreamWeaveHttpRequestChunk) -> StreamWeaveHttpResponse {
      self.response.clone()
    }
  }

  #[test]
  fn test_router_creation() {
    let router = HttpRouterTransformer::new();
    assert!(router.routes.is_empty());
    assert!(router.handlers.is_empty());
    assert!(router.fallback_handler.is_none());
  }

  #[test]
  fn test_add_route() {
    let pattern = RoutePattern::new(Method::GET, "/users/{id}").unwrap();
    let handler = Arc::new(TestHandler {
      response: StreamWeaveHttpResponse::ok("test".into()),
    });

    let router =
      HttpRouterTransformer::new().add_route(pattern, "test_handler".to_string(), handler);

    assert_eq!(router.routes.len(), 1);
    assert_eq!(router.handlers.len(), 1);
  }

  #[test]
  fn test_with_fallback_handler() {
    let fallback_handler = Arc::new(TestHandler {
      response: StreamWeaveHttpResponse::not_found("Not found".into()),
    });

    let router = HttpRouterTransformer::new().with_fallback_handler(fallback_handler);

    assert!(router.fallback_handler.is_some());
  }

  #[test]
  fn test_with_config() {
    let config = TransformerConfig::default().with_name("test_router".to_string());

    let router = HttpRouterTransformer::new().with_config(config);

    assert_eq!(router.config.name(), Some("test_router".to_string()));
  }
}
