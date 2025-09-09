use http::{HeaderMap, Method, Uri, Version};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use streamweave::{
  consumers::vec::vec_consumer::VecConsumer,
  http::{
    connection_info::ConnectionInfo, http_handler::HttpHandler,
    http_request_chunk::StreamWeaveHttpRequestChunk, http_response::StreamWeaveHttpResponse,
    route_pattern::RoutePattern,
  },
  pipeline::PipelineBuilder,
  producers::vec::vec_producer::VecProducer,
  transformer::{Transformer, TransformerConfig},
  transformers::{
    http_middleware::{
      compression_transformer::{CompressionAlgorithm, CompressionConfig, CompressionTransformer},
      cors_transformer::{CorsConfig, CorsTransformer},
      logging_transformer::{LogLevel, RequestLoggingTransformer},
      rate_limit_transformer::{CustomKeyExtractor, RateLimitStrategy, RateLimitTransformer},
      response_transform_transformer::ResponseTransformTransformer,
      validation_transformer::RequestValidationTransformer,
    },
    http_response_builder::{
      builder_utils::JsonResponseBuilder,
      response_data::ResponseData,
    },
    http_router::transformer::HttpRouterTransformer,
  },
};

/// Handler for GET /api/users - returns list of all users
struct UsersHandler;

#[async_trait::async_trait]
impl HttpHandler for UsersHandler {
  async fn handle(&self, _request: StreamWeaveHttpRequestChunk) -> StreamWeaveHttpResponse {
    // Simulate some processing time
    tokio::time::sleep(Duration::from_millis(100)).await;

    let response_data = JsonResponseBuilder::with_status(http::StatusCode::OK)
      .array_field(
        "users",
        vec![
          serde_json::json!({
              "id": 1,
              "name": "Alice",
              "email": "alice@example.com"
          }),
          serde_json::json!({
              "id": 2,
              "name": "Bob",
              "email": "bob@example.com"
          }),
          serde_json::json!({
              "id": 3,
              "name": "Charlie",
              "email": "charlie@example.com"
          }),
        ],
      )
      .number_field("total", 3.0)
      .number_field("page", 1.0)
      .header("x-response-time", "100ms")
      .header("x-api-version", "v1")
      .security_headers()
      .cors_headers("https://app.example.com")
      .build();

    // Convert ResponseData to StreamWeaveHttpResponse
    match response_data {
      ResponseData::Success {
        status,
        headers,
        body,
      } => StreamWeaveHttpResponse::new(status, headers, body),
      ResponseData::Error { status, message } => {
        StreamWeaveHttpResponse::new(status, HeaderMap::new(), message.into())
      }
    }
  }
}

/// Handler for GET /api/users/{id} - returns specific user by ID
struct UserHandler;

#[async_trait::async_trait]
impl HttpHandler for UserHandler {
  async fn handle(&self, request: StreamWeaveHttpRequestChunk) -> StreamWeaveHttpResponse {
    let path = request.path();
    
    // Simulate some processing time
    tokio::time::sleep(Duration::from_millis(100)).await;

    let user_id = path.strip_prefix("/api/users/").unwrap_or("unknown");
    let response_data = JsonResponseBuilder::with_status(http::StatusCode::OK)
      .string_field("id", user_id)
      .string_field("name", &format!("User {}", user_id))
      .string_field("email", &format!("user{}@example.com", user_id))
      .header("x-response-time", "100ms")
      .header("x-api-version", "v1")
      .security_headers()
      .cors_headers("https://app.example.com")
      .build();

    // Convert ResponseData to StreamWeaveHttpResponse
    match response_data {
      ResponseData::Success {
        status,
        headers,
        body,
      } => StreamWeaveHttpResponse::new(status, headers, body),
      ResponseData::Error { status, message } => {
        StreamWeaveHttpResponse::new(status, HeaderMap::new(), message.into())
      }
    }
  }
}

/// Handler for POST /api/users - creates a new user
struct CreateUserHandler;

#[async_trait::async_trait]
impl HttpHandler for CreateUserHandler {
  async fn handle(&self, _request: StreamWeaveHttpRequestChunk) -> StreamWeaveHttpResponse {
    // Simulate some processing time
    tokio::time::sleep(Duration::from_millis(100)).await;

    let response_data = JsonResponseBuilder::with_status(http::StatusCode::CREATED)
      .string_field("message", "User created successfully")
      .number_field("id", 4.0)
      .string_field("status", "created")
      .header("x-response-time", "100ms")
      .header("x-api-version", "v1")
      .security_headers()
      .cors_headers("https://app.example.com")
      .build();

    // Convert ResponseData to StreamWeaveHttpResponse
    match response_data {
      ResponseData::Success {
        status,
        headers,
        body,
      } => StreamWeaveHttpResponse::new(status, headers, body),
      ResponseData::Error { status, message } => {
        StreamWeaveHttpResponse::new(status, HeaderMap::new(), message.into())
      }
    }
  }
}

/// Handler for GET /health - health check endpoint
struct HealthHandler;

#[async_trait::async_trait]
impl HttpHandler for HealthHandler {
  async fn handle(&self, _request: StreamWeaveHttpRequestChunk) -> StreamWeaveHttpResponse {
    let response_data = JsonResponseBuilder::with_status(http::StatusCode::OK)
      .string_field("status", "healthy")
      .string_field("timestamp", "2024-01-15T10:30:00Z")
      .security_headers()
      .cors_headers("https://monitoring.example.com")
      .build();

    // Convert ResponseData to StreamWeaveHttpResponse
    match response_data {
      ResponseData::Success {
        status,
        headers,
        body,
      } => StreamWeaveHttpResponse::new(status, headers, body),
      ResponseData::Error { status, message } => {
        StreamWeaveHttpResponse::new(status, HeaderMap::new(), message.into())
      }
    }
  }
}

/// Handler for GET /version - version information endpoint
struct VersionHandler;

#[async_trait::async_trait]
impl HttpHandler for VersionHandler {
  async fn handle(&self, _request: StreamWeaveHttpRequestChunk) -> StreamWeaveHttpResponse {
    let response_data = JsonResponseBuilder::with_status(http::StatusCode::OK)
      .string_field("version", "1.0.0")
      .string_field("build", "2024-01-15")
      .security_headers()
      .cors_headers("https://monitoring.example.com")
      .build();

    // Convert ResponseData to StreamWeaveHttpResponse
    match response_data {
      ResponseData::Success {
        status,
        headers,
        body,
      } => StreamWeaveHttpResponse::new(status, headers, body),
      ResponseData::Error { status, message } => {
        StreamWeaveHttpResponse::new(status, HeaderMap::new(), message.into())
      }
    }
  }
}

/// Handler for GET /metrics - metrics endpoint
struct MetricsHandler;

#[async_trait::async_trait]
impl HttpHandler for MetricsHandler {
  async fn handle(&self, _request: StreamWeaveHttpRequestChunk) -> StreamWeaveHttpResponse {
    let response_data = JsonResponseBuilder::with_status(http::StatusCode::OK)
      .number_field("requests_total", 1234.0)
      .number_field("requests_per_second", 45.6)
      .string_field("average_response_time", "120ms")
      .number_field("error_rate", 0.02)
      .security_headers()
      .cors_headers("https://monitoring.example.com")
      .build();

    // Convert ResponseData to StreamWeaveHttpResponse
    match response_data {
      ResponseData::Success {
        status,
        headers,
        body,
      } => StreamWeaveHttpResponse::new(status, headers, body),
      ResponseData::Error { status, message } => {
        StreamWeaveHttpResponse::new(status, HeaderMap::new(), message.into())
      }
    }
  }
}

/// Create test requests with different scenarios
fn create_test_requests() -> Vec<StreamWeaveHttpRequestChunk> {
  let mut requests = Vec::new();

  // Helper function to create a request
  let create_request = |method: Method, path: &str, headers: HeaderMap, body: &str| {
    let uri: Uri = format!("https://api.example.com{}", path).parse().unwrap();
    let connection_info = ConnectionInfo::new(
      SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
      SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 3000),
      Version::HTTP_11,
    );

    StreamWeaveHttpRequestChunk::new(
      method,
      uri,
      headers,
      bytes::Bytes::from(body.to_string()),
      connection_info,
      true,
    )
  };

  // 1. Public health check (no auth required)
  let mut headers = HeaderMap::new();
  headers.insert("user-agent", "Health Checker 1.0".parse().unwrap());
  headers.insert("origin", "https://monitoring.example.com".parse().unwrap());
  requests.push(create_request(Method::GET, "/health", headers, ""));

  // 2. Public version endpoint
  let mut headers = HeaderMap::new();
  headers.insert("user-agent", "Version Checker 1.0".parse().unwrap());
  headers.insert("origin", "https://monitoring.example.com".parse().unwrap());
  requests.push(create_request(Method::GET, "/version", headers, ""));

  // 3. POST request with large JSON body (for compression testing)
  let mut headers = HeaderMap::new();
  headers.insert("content-type", "application/json".parse().unwrap());
  headers.insert("accept-encoding", "gzip, deflate".parse().unwrap());
  headers.insert("origin", "https://app.example.com".parse().unwrap());
  let large_json = format!(r#"{{"users": [{}]}}"#, 
        (1..100).map(|i| format!(r#"{{"id": {}, "name": "User {}", "data": "This is a large amount of data for user {} to test compression"}}"#, i, i, i))
        .collect::<Vec<_>>()
        .join(", ")
    );
  requests.push(create_request(
    Method::POST,
    "/api/users",
    headers,
    &large_json,
  ));

  // 4. CORS preflight request
  let mut headers = HeaderMap::new();
  headers.insert("origin", "https://app.example.com".parse().unwrap());
  headers.insert("access-control-request-method", "POST".parse().unwrap());
  headers.insert(
    "access-control-request-headers",
    "content-type,authorization".parse().unwrap(),
  );
  requests.push(create_request(Method::OPTIONS, "/api/users", headers, ""));

  // 5. API request (will be handled by user handler)
  let mut headers = HeaderMap::new();
  headers.insert("user-agent", "API Client 1.0".parse().unwrap());
  headers.insert("accept", "application/json".parse().unwrap());
  headers.insert("origin", "https://app.example.com".parse().unwrap());
  requests.push(create_request(Method::GET, "/api/users", headers, ""));

  requests
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Initialize logging
  tracing_subscriber::fmt::init();

  println!("🚀 StreamWeave HTTP Middleware Pipeline Example");
  println!("===============================================");
  println!("This example demonstrates the full StreamWeave pipeline architecture:");
  println!("• Complete request-to-response pipeline with all middleware");
  println!("• HTTP Router for request routing");
  println!("• HTTP Response Builder for structured responses");
  println!("• All transformers working together in a single stream");
  println!();

  // Create route patterns
  let api_users_route = RoutePattern::new(Method::GET, "/api/users")?;
  let api_user_route = RoutePattern::new(Method::GET, "/api/users/{id}")?;
  let api_create_user_route = RoutePattern::new(Method::POST, "/api/users")?;
  let health_route = RoutePattern::new(Method::GET, "/health")?;
  let version_route = RoutePattern::new(Method::GET, "/version")?;
  let metrics_route = RoutePattern::new(Method::GET, "/metrics")?;

  // Create individual handlers
  let users_handler = Arc::new(UsersHandler);
  let user_handler = Arc::new(UserHandler);
  let create_user_handler = Arc::new(CreateUserHandler);
  let health_handler = Arc::new(HealthHandler);
  let version_handler = Arc::new(VersionHandler);
  let metrics_handler = Arc::new(MetricsHandler);

  // Build the HTTP router transformer
  let mut router = HttpRouterTransformer::new()
    .add_route(
      api_users_route,
      "api_users".to_string(),
      users_handler,
    )?
    .add_route(api_user_route, "api_user".to_string(), user_handler)?
    .add_route(
      api_create_user_route,
      "api_create_user".to_string(),
      create_user_handler,
    )?
    .add_route(health_route, "health".to_string(), health_handler)?
    .add_route(version_route, "version".to_string(), version_handler)?
    .add_route(metrics_route, "metrics".to_string(), metrics_handler)?;

  router.set_config(TransformerConfig::default().with_name("http_router".to_string()));

  // Note: HTTP Response Builder is used internally by the handlers

  // Create middleware transformers
  let cors_config = CorsConfig {
    allowed_origins: vec![
      "https://app.example.com".to_string(),
      "https://monitoring.example.com".to_string(),
    ],
    allowed_methods: vec![
      Method::GET,
      Method::POST,
      Method::PUT,
      Method::DELETE,
      Method::OPTIONS,
    ],
    allowed_headers: vec![
      "content-type".to_string(),
      "authorization".to_string(),
      "x-api-key".to_string(),
      "user-agent".to_string(),
      "accept".to_string(),
      "origin".to_string(),
    ],
    exposed_headers: vec!["x-response-time".to_string(), "x-api-version".to_string()],
    allow_credentials: true,
    max_age: Some(3600),
  };

  let mut cors_transformer = CorsTransformer::new().with_config(cors_config);
  cors_transformer.set_config(TransformerConfig::default().with_name("cors".to_string()));

  let mut request_logging = RequestLoggingTransformer::new()
    .with_log_level(LogLevel::Info)
    .with_include_body(false);
  request_logging.set_config(TransformerConfig::default().with_name("request_logging".to_string()));

  // Note: Response logging is handled by the logging middleware

  let compression_config = CompressionConfig {
    enabled_algorithms: vec![
      CompressionAlgorithm::Gzip,
      CompressionAlgorithm::Deflate,
      CompressionAlgorithm::Brotli,
    ],
    min_size: 1024,                   // Only compress if body is larger than 1KB
    max_size: Some(10 * 1024 * 1024), // 10MB max
    content_types: std::collections::HashSet::from([
      "application/json".to_string(),
      "text/plain".to_string(),
      "text/html".to_string(),
    ]),
    exclude_content_types: std::collections::HashSet::new(),
    compression_level: 6,
  };

  let mut compression_transformer = CompressionTransformer::new().with_config(compression_config);
  compression_transformer
    .set_config(TransformerConfig::default().with_name("compression".to_string()));

  // Create rate limit key extractor
  let key_extractor = CustomKeyExtractor::new(|request| {
    request
      .headers
      .get("x-api-key")
      .and_then(|h| h.to_str().ok())
      .map(|s| s.to_string())
      .unwrap_or_else(|| request.connection_info.remote_addr.to_string())
  });

  let mut rate_limit_transformer = RateLimitTransformer::new(
    RateLimitStrategy::TokenBucket {
      capacity: 10,
      refill_rate: 2.0, // 2 requests per second
      refill_period: Duration::from_secs(1),
    },
    Box::new(key_extractor),
  );
  rate_limit_transformer
    .set_config(TransformerConfig::default().with_name("rate_limit".to_string()));

  let mut validation_transformer = RequestValidationTransformer::new();
  validation_transformer
    .set_config(TransformerConfig::default().with_name("validation".to_string()));

  let mut response_transform_transformer = ResponseTransformTransformer::new();
  response_transform_transformer
    .set_config(TransformerConfig::default().with_name("response_transform".to_string()));

  // Create test requests
  let test_requests = create_test_requests();

  println!(
    "📝 Processing {} test requests through complete StreamWeave pipeline",
    test_requests.len()
  );
  println!("==================================================================");

  // Process all requests through the complete pipeline
  let request_producer =
    VecProducer::new(test_requests).with_name("http_request_producer".to_string());

  let response_consumer =
    VecConsumer::<StreamWeaveHttpResponse>::new().with_name("response_collector".to_string());

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

  let pipeline_result = pipeline.run().await;

  match pipeline_result {
    Ok(((), consumer)) => {
      let responses = consumer.into_vec();
      println!("\n🎉 StreamWeave Pipeline Processing Complete!");
      println!("=============================================");

      for (i, response) in responses.into_iter().enumerate() {
        println!("\n--- Response {} ---", i + 1);
        println!("Status: {}", response.status);
        println!("Headers: {}", response.headers.len());
        println!("Body size: {} bytes", response.body.len());

        // Show some key headers
        if let Some(content_type) = response.headers.get("content-type") {
          println!(
            "Content-Type: {}",
            content_type.to_str().unwrap_or("invalid")
          );
        }
        if let Some(encoding) = response.headers.get("content-encoding") {
          println!(
            "Content-Encoding: {}",
            encoding.to_str().unwrap_or("invalid")
          );
        }
        if let Some(cors) = response.headers.get("access-control-allow-origin") {
          println!("CORS Origin: {}", cors.to_str().unwrap_or("invalid"));
        }

        // Show a preview of the response body
        let body_preview = String::from_utf8_lossy(&response.body);
        println!(
          "Body preview: {}",
          &body_preview[..body_preview.len().min(100)]
        );
      }
    }
    Err(e) => {
      println!("❌ Pipeline error: {}", e);
    }
  }

  println!("\n🎉 HTTP Middleware Pipeline example completed successfully!");
  println!("\nKey Features Demonstrated:");
  println!("• 🌐 CORS: Cross-Origin Resource Sharing handling");
  println!("• 📝 Logging: Request and response logging with structured data");
  println!("• 🗜️  Compression: Automatic response compression (Gzip, Deflate, Brotli)");
  println!("• ⏱️  Rate Limiting: Token bucket rate limiting per API key");
  println!("• ✅ Validation: Request validation (headers, body size, content type)");
  println!("• 🔄 Response Transformation: Security headers, caching, JSON minification");
  println!("• 🚦 Pipeline: All middleware working together in a single stream");
  println!(
    "• 🆕 HTTP Response Builder: Structured response generation with security headers and CORS"
  );
  println!("• 🛣️  HTTP Router: Request routing with path parameters and fallback handling");
  println!("• 🔗 StreamWeave Architecture: Complete request-to-response pipeline in one stream");

  Ok(())
}
