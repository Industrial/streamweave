//! # HTTP Server Middleware
//!
//! This module provides middleware configurations and utilities for Axum HTTP servers
//! using StreamWeave. It includes common middleware patterns like CORS, authentication,
//! logging, and rate limiting.
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::http_server::middleware;
//! use axum::Router;
//!
//! let app = Router::new()
//!     .layer(middleware::cors_layer())
//!     .layer(middleware::logging_layer());
//! ```

#[cfg(feature = "http-server")]
use axum::http::HeaderValue;
#[cfg(feature = "http-server")]
use tower_http::{cors::CorsLayer, trace::TraceLayer};

/// Creates a CORS middleware layer with permissive defaults.
///
/// This middleware allows all origins, methods, and headers by default.
/// For production use, you should configure it more restrictively.
///
/// ## Example
///
/// ```rust,no_run
/// use streamweave::http_server::middleware;
/// use axum::Router;
///
/// let app = Router::new()
///     .layer(middleware::cors_layer());
/// ```
#[cfg(feature = "http-server")]
pub fn cors_layer() -> CorsLayer {
  CorsLayer::new()
    .allow_origin(tower_http::cors::Any)
    .allow_methods(tower_http::cors::Any)
    .allow_headers(tower_http::cors::Any)
    .allow_credentials(true)
}

/// Creates a CORS middleware layer with specific allowed origins.
///
/// ## Example
///
/// ```rust,no_run
/// use streamweave::http_server::middleware;
/// use axum::Router;
///
/// let app = Router::new()
///     .layer(middleware::cors_layer_with_origins(vec![
///         "http://localhost:3000".parse().unwrap(),
///         "https://example.com".parse().unwrap(),
///     ]));
/// ```
#[cfg(feature = "http-server")]
pub fn cors_layer_with_origins(origins: Vec<HeaderValue>) -> CorsLayer {
  CorsLayer::new()
    .allow_origin(tower_http::cors::AllowOrigin::list(origins))
    .allow_methods(tower_http::cors::Any)
    .allow_headers(tower_http::cors::Any)
    .allow_credentials(true)
}

/// Creates a logging middleware layer using tracing.
///
/// This middleware logs all incoming requests and responses using the `tracing` crate.
/// It includes request method, path, status code, and duration.
///
/// ## Example
///
/// ```rust,no_run
/// use streamweave::http_server::middleware;
/// use axum::Router;
///
/// let app = Router::new()
///     .layer(middleware::logging_layer());
/// ```
#[cfg(feature = "http-server")]
pub fn logging_layer()
-> TraceLayer<tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>>
{
  // Use default TraceLayer - it will automatically log requests/responses
  // Users can customize further if needed by calling methods on the returned layer
  TraceLayer::new_for_http()
}

/// Example authentication middleware function that extracts Bearer tokens.
///
/// This is a basic example. For production use, you should implement proper
/// token validation and user authentication.
///
/// ## Example
///
/// ```rust,no_run
/// use streamweave::http_server::middleware;
/// use axum::{Router, extract::Request, middleware::Next, response::Response};
/// use axum::body::Body;
///
/// async fn auth_middleware(request: Request, next: Next) -> Response<Body> {
///     // Extract and validate token
///     if let Some(auth_header) = request.headers().get("authorization") {
///         if let Ok(auth_str) = auth_header.to_str() {
///             if auth_str.starts_with("Bearer ") {
///                 let token = &auth_str[7..];
///                 // Validate token here
///                 return next.run(request).await;
///             }
///         }
///     }
///     
///     // Return 401 if no valid token
///     axum::response::Response::builder()
///         .status(axum::http::StatusCode::UNAUTHORIZED)
///         .body(Body::from("Unauthorized"))
///         .unwrap()
/// }
///
/// let app = Router::new()
///     .route_layer(axum::middleware::from_fn(auth_middleware));
/// ```
#[cfg(feature = "http-server")]
pub async fn example_auth_middleware(
  request: axum::extract::Request,
  next: axum::middleware::Next,
) -> axum::response::Response<axum::body::Body> {
  // Extract and validate token
  if let Some(auth_header) = request.headers().get("authorization")
    && let Ok(auth_str) = auth_header.to_str()
    && let Some(_token) = auth_str.strip_prefix("Bearer ")
  {
    // Validate token here - for this example, we'll just pass through
    return next.run(request).await;
  }

  // Return 401 if no valid token
  axum::response::Response::builder()
    .status(axum::http::StatusCode::UNAUTHORIZED)
    .body(axum::body::Body::from("Unauthorized"))
    .unwrap()
}

/// Creates a rate limiting middleware layer.
///
/// This is a basic example. For production use, you should use a proper
/// rate limiting library like `tower-governor` or `tower-ratelimit`.
///
/// ## Example
///
/// ```rust,no_run
/// use streamweave::http_server::middleware;
/// use axum::Router;
///
/// // Note: This is a placeholder - implement actual rate limiting
/// // using a library like tower-governor
/// ```
#[cfg(feature = "http-server")]
pub fn rate_limit_layer() -> tower::layer::util::Identity {
  // Placeholder - users should implement rate limiting based on their needs
  // For example, using tower-governor:
  // use tower_governor::{Governor, GovernorConfigBuilder};
  // let config = Box::new(GovernorConfigBuilder::default().build().unwrap());
  // Governor::new(config)

  // For now, return an identity layer (no-op)
  tower::layer::util::Identity::new()
}

/// Creates a middleware stack with common middleware applied.
///
/// This applies CORS, logging, and other common middleware in the correct order.
///
/// ## Example
///
/// ```rust,no_run
/// use streamweave::http_server::middleware;
/// use axum::Router;
///
/// let app = Router::new()
///     .layer(middleware::common_middleware_stack());
/// ```
#[cfg(feature = "http-server")]
pub fn common_middleware_stack()
-> impl tower::Layer<axum::routing::Router> + Clone + Send + Sync + 'static {
  use tower::ServiceBuilder;
  ServiceBuilder::new()
    .layer(cors_layer())
    .layer(logging_layer())
}
