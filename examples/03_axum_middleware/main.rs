use axum::{
  Router,
  http::{HeaderMap, Method, StatusCode, Uri},
  response::Json,
  routing::any,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

use http::Method as HttpMethod;
use std::collections::HashMap;
use std::time::Duration;
use streamweave::{
  consumers::vec::vec_consumer::VecConsumer,
  http::{
    connection_info::ConnectionInfo, http_request_chunk::StreamWeaveHttpRequestChunk,
    http_response::StreamWeaveHttpResponse, route_pattern::RoutePattern,
  },
  pipeline::PipelineBuilder,
  producers::vec::vec_producer::VecProducer,
  transformers::{
    http_middleware::{
      compression_transformer::CompressionTransformer,
      cors_transformer::CorsTransformer,
      logging_transformer::RequestLoggingTransformer,
      rate_limit_transformer::{IpKeyExtractor, RateLimitStrategy, RateLimitTransformer},
      response_transform_transformer::ResponseTransformTransformer,
      validation_transformer::RequestValidationTransformer,
    },
    http_router::http_router_transformer::HttpRouterTransformer,
  },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
  id: u32,
  name: String,
  email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiResponse<T> {
  data: T,
  status: String,
  timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HealthResponse {
  status: String,
  timestamp: String,
  uptime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VersionResponse {
  version: String,
  build: String,
  rust_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetricsResponse {
  requests_total: u64,
  requests_per_second: f64,
  average_response_time: String,
  error_rate: f64,
  memory_usage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UsersResponse {
  users: Vec<User>,
  total: usize,
  page: u32,
}

#[derive(Clone)]
struct AppState {}

// Single handler that uses StreamWeave pipeline for all requests
async fn streamweave_handler(
  method: Method,
  uri: Uri,
  headers: HeaderMap,
  body: axum::body::Bytes,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
  println!("🌐 [axum] Received request: {} {}", method, uri.path());

  // Convert Axum body to Vec<u8>
  let body_vec = body.to_vec();

  // Process through StreamWeave pipeline
  match process_request_with_streamweave(method, uri, headers, body_vec).await {
    Ok(response) => {
      // Convert StreamWeave response to Axum response
      let body_str = String::from_utf8_lossy(&response.body);
      match serde_json::from_str::<serde_json::Value>(&body_str) {
        Ok(json) => {
          println!("✅ [axum] Successfully processed request through StreamWeave pipeline");
          Ok(Json(json))
        }
        Err(_) => {
          // If not JSON, return as text
          Ok(Json(serde_json::Value::String(body_str.to_string())))
        }
      }
    }
    Err(e) => {
      println!("❌ [axum] StreamWeave pipeline error: {}", e);
      Err((StatusCode::INTERNAL_SERVER_ERROR, e))
    }
  }
}

// StreamWeave middleware pipeline with routing
async fn process_request_with_streamweave(
  method: Method,
  uri: Uri,
  headers: HeaderMap,
  body: Vec<u8>,
) -> Result<StreamWeaveHttpResponse, String> {
  println!(
    "🔄 [streamweave] Processing request: {} {}",
    method,
    uri.path()
  );

  let request_chunk = StreamWeaveHttpRequestChunk {
    method: method.clone(),
    uri: uri.clone(),
    headers: headers.clone(),
    chunk: body.clone().into(),
    path_params: HashMap::new(),
    query_params: HashMap::new(),
    connection_info: ConnectionInfo::new(
      "127.0.0.1:0".parse().unwrap(),
      "127.0.0.1:3004".parse().unwrap(),
      http::Version::HTTP_11,
    ),
    is_final: true,
    request_id: Uuid::new_v4(),
    connection_id: Uuid::new_v4(),
  };

  let cors_transformer = CorsTransformer::new();
  let request_logging = RequestLoggingTransformer::new();
  let validation_transformer = RequestValidationTransformer::new();
  let rate_limit_transformer = RateLimitTransformer::new(
    RateLimitStrategy::TokenBucket {
      capacity: 100,
      refill_rate: 10.0,
      refill_period: Duration::from_secs(1),
    },
    Box::new(IpKeyExtractor),
  );

  let router = HttpRouterTransformer::new()
    .add_route(
      RoutePattern::new(HttpMethod::GET, "/").unwrap(),
      "root_handler".to_string(),
    )
    .unwrap()
    .add_route(
      RoutePattern::new(HttpMethod::GET, "/health").unwrap(),
      "health_handler".to_string(),
    )
    .unwrap()
    .add_route(
      RoutePattern::new(HttpMethod::GET, "/version").unwrap(),
      "version_handler".to_string(),
    )
    .unwrap()
    .add_route(
      RoutePattern::new(HttpMethod::GET, "/metrics").unwrap(),
      "metrics_handler".to_string(),
    )
    .unwrap()
    .add_route(
      RoutePattern::new(HttpMethod::GET, "/api/users").unwrap(),
      "users_handler".to_string(),
    )
    .unwrap()
    .add_route(
      RoutePattern::new(HttpMethod::GET, "/api/users/{id}").unwrap(),
      "user_by_id_handler".to_string(),
    )
    .unwrap()
    .add_route(
      RoutePattern::new(HttpMethod::POST, "/api/users").unwrap(),
      "create_user_handler".to_string(),
    )
    .unwrap();

  let compression_transformer = CompressionTransformer::new();
  let response_transform_transformer = ResponseTransformTransformer::new();

  let request_producer =
    VecProducer::new(vec![request_chunk]).with_name("request_producer".to_string());
  let response_consumer = VecConsumer::new().with_name("response_consumer".to_string());

  let pipeline = PipelineBuilder::new()
    .producer(request_producer)
    .transformer(cors_transformer)
    .transformer(request_logging)
    .transformer(validation_transformer)
    .transformer(rate_limit_transformer)
    .transformer(router)
    .transformer(compression_transformer)
    .transformer(response_transform_transformer)
    ._consumer(response_consumer);

  match pipeline.run().await {
    Ok(((), consumer)) => {
      let responses = consumer.into_vec();

      if let Some(response) = responses.into_iter().next() {
        Ok(response)
      } else {
        Err("No response generated".to_string())
      }
    }
    Err(e) => Err(format!("Pipeline error: {}", e)),
  }
}

#[tokio::main]
async fn main() {
  println!("🚀 StreamWeave + Axum HTTP Middleware Example");
  println!("=============================================");
  println!("Demonstrates StreamWeave middleware pipeline integrated with Axum HTTP server");
  println!("• Axum handles HTTP protocol and routing");
  println!("• StreamWeave handles middleware pipeline");
  println!("• Clean separation of concerns");
  println!("• Production-ready HTTP server");
  println!();

  // Create application state
  let state = AppState {};

  // Build the router - all requests go through StreamWeave pipeline
  let app = Router::new()
    .route("/{*path}", any(streamweave_handler))
    .layer(ServiceBuilder::new().layer(CorsLayer::permissive()))
    .with_state(state);

  // Start the server
  let addr = SocketAddr::from(([127, 0, 0, 1], 3004));
  println!("🌐 Starting Axum server on {}", addr);
  println!("📋 Available endpoints:");
  println!("   GET  /                    - Root endpoint");
  println!("   GET  /health              - Health check");
  println!("   GET  /version             - Version information");
  println!("   GET  /metrics             - Metrics endpoint");
  println!("   GET  /api/users           - List all users");
  println!("   GET  /api/users/:id       - Get user by ID");
  println!("   POST /api/users           - Create new user");
  println!();
  println!("🔗 Test endpoints in your browser:");
  println!("   http://localhost:3004/");
  println!("   http://localhost:3004/health");
  println!("   http://localhost:3004/version");
  println!("   http://localhost:3004/metrics");
  println!("   http://localhost:3004/api/users");
  println!();
  println!("🎉 All endpoints use Axum + StreamWeave architecture!");
  println!("   - Axum: HTTP protocol handling, routing, middleware");
  println!("   - StreamWeave: Business logic pipeline, transformations");
  println!("   - Clean separation of concerns");
  println!("   - Production-ready and maintainable");
  println!();
  println!("💡 This example demonstrates the recommended approach:");
  println!("   - Use Axum for HTTP server functionality");
  println!("   - Use StreamWeave for business logic and data processing");
  println!("   - Avoid custom HTTP protocol implementation");
  println!("   - Focus on what each framework does best");

  let listener = TcpListener::bind(addr).await.unwrap();
  axum::serve(listener, app).await.unwrap();
}
