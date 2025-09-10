use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::http::{
  http_request_chunk::StreamWeaveHttpRequestChunk, http_response::StreamWeaveHttpResponse,
  route_pattern::RoutePattern,
};

use crate::input::Input;
use crate::output::Output;
use crate::transformer::{Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::StreamExt;
use http::Method;
use std::collections::HashMap;

/// HTTP Router Transformer that routes requests to appropriate handlers
#[derive(Clone)]
pub struct HttpRouterTransformer {
  routes: HashMap<RoutePattern, String>, // Pattern -> Handler ID
  config: TransformerConfig<StreamWeaveHttpRequestChunk>,
}

impl HttpRouterTransformer {
  /// Create a new HTTP Router Transformer
  pub fn new() -> Self {
    Self {
      routes: HashMap::new(),
      config: TransformerConfig::default(),
    }
  }

  /// Add a route to the router
  pub fn add_route(
    mut self,
    pattern: RoutePattern,
    handler_id: String,
  ) -> Result<Self, Box<dyn std::error::Error>> {
    if self.routes.contains_key(&pattern) {
      return Err("Route already exists".into());
    }
    self.routes.insert(pattern, handler_id);
    Ok(self)
  }

  /// Set the configuration
  pub fn set_config(mut self, config: TransformerConfig<StreamWeaveHttpRequestChunk>) -> Self {
    self.config = config;
    self
  }
}

impl Default for HttpRouterTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Input for HttpRouterTransformer {
  type Input = StreamWeaveHttpRequestChunk;
  type InputStream =
    std::pin::Pin<Box<dyn futures::Stream<Item = StreamWeaveHttpRequestChunk> + Send>>;
}

impl Output for HttpRouterTransformer {
  type Output = StreamWeaveHttpResponse;
  type OutputStream =
    std::pin::Pin<Box<dyn futures::Stream<Item = StreamWeaveHttpResponse> + Send>>;
}

#[async_trait]
impl Transformer for HttpRouterTransformer {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    #[allow(clippy::mutable_key_type)]
    let routes = self.routes.clone();
    let transformer_name = self.config.name().unwrap_or("http_router".to_string());

    Box::pin(async_stream::stream! {
        let mut input = input;

        while let Some(request) = input.next().await {
            let method = request.method.clone();
            let path = request.uri.path();

            println!("🛣️  [{}] Routing request: {} {}", transformer_name, method, path);

            // Find matching route
            let (handler_id, _path_params) = match Self::find_route_static(&routes, &method, path) {
                Some((id, params)) => {
                    println!("   ✅ [{}] Route matched: {} -> {} with params: {:?}",
                        transformer_name, path, id, params);
                    (id, params)
                },
                None => {
                    println!("   ❌ [{}] No route found for {} {}", transformer_name, method, path);
                    // No route found, return 404
                    let response = StreamWeaveHttpResponse::not_found(
                        format!("No route found for {} {}", method, path).into()
                    );
                    yield response;
                    continue;
                }
            };

            // Generate response based on handler ID
            let response = match handler_id.as_str() {
                "root_handler" => StreamWeaveHttpResponse::ok(
                    "Hello, World! This is StreamWeave + Axum!".into()
                ),
                "health_handler" => {
                    let health_response = serde_json::json!({
                        "data": {
                            "status": "healthy",
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                            "uptime": "24h 15m 30s"
                        },
                        "status": "success",
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });
                    StreamWeaveHttpResponse::ok(serde_json::to_string(&health_response).unwrap().into())
                },
                "version_handler" => {
                    let version_response = serde_json::json!({
                        "data": {
                            "version": "1.0.0",
                            "build": "2024-01-15",
                            "rust_version": "1.89.0"
                        },
                        "status": "success",
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });
                    StreamWeaveHttpResponse::ok(serde_json::to_string(&version_response).unwrap().into())
                },
                "metrics_handler" => {
                    let metrics_response = serde_json::json!({
                        "data": {
                            "requests_total": 1234,
                            "requests_per_second": 45.6,
                            "average_response_time": "120ms",
                            "error_rate": 0.02,
                            "memory_usage": "45.2 MB"
                        },
                        "status": "success",
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });
                    StreamWeaveHttpResponse::ok(serde_json::to_string(&metrics_response).unwrap().into())
                },
                "users_handler" => {
                    let users_response = serde_json::json!({
                        "data": {
                            "users": [
                                {"id": 1, "name": "Alice", "email": "alice@example.com"},
                                {"id": 2, "name": "Bob", "email": "bob@example.com"},
                                {"id": 3, "name": "Charlie", "email": "charlie@example.com"}
                            ],
                            "total": 3,
                            "page": 1
                        },
                        "status": "success",
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });
                    StreamWeaveHttpResponse::ok(serde_json::to_string(&users_response).unwrap().into())
                },
                "user_by_id_handler" => {
                    let user_id = _path_params.get("id").map(|s| s.as_str()).unwrap_or("unknown");
                    let user_response = serde_json::json!({
                        "data": {
                            "id": user_id.parse::<u32>().unwrap_or(0),
                            "name": format!("User {}", user_id),
                            "email": format!("user{}@example.com", user_id)
                        },
                        "status": "success",
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });
                    StreamWeaveHttpResponse::ok(serde_json::to_string(&user_response).unwrap().into())
                },
                "create_user_handler" => {
                    let create_response = serde_json::json!({
                        "data": {
                            "id": 999,
                            "name": "New User",
                            "email": "newuser@example.com"
                        },
                        "status": "created",
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });
                    StreamWeaveHttpResponse::ok(serde_json::to_string(&create_response).unwrap().into())
                },
                _ => StreamWeaveHttpResponse::not_found(
                    format!("Handler not found: {}", handler_id).into()
                ),
            };

            println!("   📤 [{}] Generated response for handler: {}", transformer_name, handler_id);
            yield response;
        }
    })
  }

  fn set_config_impl(&mut self, config: TransformerConfig<StreamWeaveHttpRequestChunk>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<StreamWeaveHttpRequestChunk> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<StreamWeaveHttpRequestChunk> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<StreamWeaveHttpRequestChunk>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(
    &self,
    item: Option<StreamWeaveHttpRequestChunk>,
  ) -> ErrorContext<StreamWeaveHttpRequestChunk> {
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
        .unwrap_or_else(|| "http_router".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

impl HttpRouterTransformer {
  /// Static method to find route (used in async context)
  #[allow(clippy::mutable_key_type)]
  fn find_route_static(
    routes: &HashMap<RoutePattern, String>,
    method: &Method,
    path: &str,
  ) -> Option<(String, HashMap<String, String>)> {
    let mut best_match: Option<(String, HashMap<String, String>, u32)> = None;

    for (pattern, handler_id) in routes {
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

/// Error types for the HTTP Router
#[derive(Debug, Clone, PartialEq)]
pub enum RouterError {
  RouteConflict {
    existing: RoutePattern,
    new: RoutePattern,
  },
  HandlerNotFound(String),
  InvalidRoute(String),
}

impl std::fmt::Display for RouterError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      RouterError::RouteConflict { existing, new } => {
        write!(
          f,
          "Route conflict: {} conflicts with {}",
          new.path, existing.path
        )
      }
      RouterError::HandlerNotFound(id) => {
        write!(f, "Handler not found: {}", id)
      }
      RouterError::InvalidRoute(msg) => {
        write!(f, "Invalid route: {}", msg)
      }
    }
  }
}

impl std::error::Error for RouterError {}

#[cfg(test)]
mod tests {
  use super::*;

  use http::Method;

  #[tokio::test]
  async fn test_router_creation() {
    let router = HttpRouterTransformer::new();
    assert!(router.routes.is_empty());
  }

  #[tokio::test]
  async fn test_add_route() {
    let pattern = RoutePattern::new(Method::GET, "/users/{id}").unwrap();
    let router = HttpRouterTransformer::new()
      .add_route(pattern, "test_handler".to_string())
      .unwrap();

    assert_eq!(router.routes.len(), 1);
  }

  #[tokio::test]
  async fn test_route_conflict() {
    let pattern1 = RoutePattern::new(Method::GET, "/users/{id}").unwrap();
    let pattern2 = RoutePattern::new(Method::GET, "/users/{id}").unwrap();

    let router = HttpRouterTransformer::new()
      .add_route(pattern1, "handler1".to_string())
      .unwrap();

    let result = router.add_route(pattern2, "handler2".to_string());
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_find_route() {
    let pattern = RoutePattern::new(Method::GET, "/users/{id}").unwrap();

    let mut routes = HashMap::new();
    routes.insert(pattern, "test_handler".to_string());

    let (handler_id, params) =
      HttpRouterTransformer::find_route_static(&routes, &Method::GET, "/users/123").unwrap();
    assert_eq!(handler_id, "test_handler");
    assert_eq!(params.get("id"), Some(&"123".to_string()));
  }

  #[tokio::test]
  async fn test_no_route_found() {
    let router = HttpRouterTransformer::new();
    let result =
      HttpRouterTransformer::find_route_static(&router.routes, &Method::GET, "/nonexistent");
    assert!(result.is_none());
  }

  #[tokio::test]
  async fn test_router_error_display() {
    let error = RouterError::HandlerNotFound("test".to_string());
    assert_eq!(error.to_string(), "Handler not found: test");

    let error = RouterError::InvalidRoute("invalid path".to_string());
    assert_eq!(error.to_string(), "Invalid route: invalid path");
  }
}
