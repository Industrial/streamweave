use futures::StreamExt;
use http::{HeaderMap, Method, Uri, Version};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use streamweave::{
  http::{
    connection_info::ConnectionInfo, http_handler::HttpHandler,
    http_request_chunk::StreamWeaveHttpRequestChunk, http_response::StreamWeaveHttpResponse,
  },
  transformer::Transformer,
  transformers::{
    http_middleware::{
      compression_transformer::{CompressionAlgorithm, CompressionConfig, CompressionTransformer},
      cors_transformer::{CorsConfig, CorsTransformer},
      logging_transformer::{LogLevel, RequestLoggingTransformer, ResponseLoggingTransformer},
      rate_limit_transformer::{CustomKeyExtractor, RateLimitStrategy, RateLimitTransformer},
      response_transform_transformer::ResponseTransformTransformer,
      validation_transformer::RequestValidationTransformer,
    },
    http_response_builder::{
      builder_utils::{ErrorResponseBuilder, JsonResponseBuilder},
      response_data::ResponseData,
    },
  },
};

/// Example API handler that returns user data
struct UserApiHandler;

#[async_trait::async_trait]
impl HttpHandler for UserApiHandler {
  async fn handle(&self, request: StreamWeaveHttpRequestChunk) -> StreamWeaveHttpResponse {
    let path = request.path();
    let method = &request.method;

    // Simulate some processing time
    tokio::time::sleep(Duration::from_millis(100)).await;

    let response_data = match (method, path) {
      (&Method::GET, "/api/users") => JsonResponseBuilder::with_status(http::StatusCode::OK)
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
        .build(),
      (&Method::GET, path) if path.starts_with("/api/users/") => {
        let user_id = path.strip_prefix("/api/users/").unwrap_or("unknown");
        JsonResponseBuilder::with_status(http::StatusCode::OK)
          .string_field("id", user_id)
          .string_field("name", &format!("User {}", user_id))
          .string_field("email", &format!("user{}@example.com", user_id))
          .header("x-response-time", "100ms")
          .header("x-api-version", "v1")
          .security_headers()
          .cors_headers("https://app.example.com")
          .build()
      }
      (&Method::POST, "/api/users") => JsonResponseBuilder::with_status(http::StatusCode::CREATED)
        .string_field("message", "User created successfully")
        .number_field("id", 4.0)
        .string_field("status", "created")
        .header("x-response-time", "100ms")
        .header("x-api-version", "v1")
        .security_headers()
        .cors_headers("https://app.example.com")
        .build(),
      _ => ErrorResponseBuilder::new(http::StatusCode::NOT_FOUND, "Endpoint not found")
        .header("x-api-version", "v1")
        .build(),
    };

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

/// Example handler for public endpoints
struct PublicHandler;

#[async_trait::async_trait]
impl HttpHandler for PublicHandler {
  async fn handle(&self, request: StreamWeaveHttpRequestChunk) -> StreamWeaveHttpResponse {
    let path = request.path();

    let response_data = match path {
      "/health" => JsonResponseBuilder::with_status(http::StatusCode::OK)
        .string_field("status", "healthy")
        .string_field("timestamp", "2024-01-15T10:30:00Z")
        .security_headers()
        .cors_headers("https://monitoring.example.com")
        .build(),
      "/version" => JsonResponseBuilder::with_status(http::StatusCode::OK)
        .string_field("version", "1.0.0")
        .string_field("build", "2024-01-15")
        .security_headers()
        .cors_headers("https://monitoring.example.com")
        .build(),
      "/metrics" => JsonResponseBuilder::with_status(http::StatusCode::OK)
        .number_field("requests_total", 1234.0)
        .number_field("requests_per_second", 45.6)
        .string_field("average_response_time", "120ms")
        .number_field("error_rate", 0.02)
        .security_headers()
        .cors_headers("https://monitoring.example.com")
        .build(),
      _ => {
        ErrorResponseBuilder::new(http::StatusCode::NOT_FOUND, "Public endpoint not found").build()
      }
    };

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

  println!("🚀 StreamWeave HTTP Middleware Example");
  println!("=====================================");
  println!("This example demonstrates all HTTP middleware transformers working together:");
  println!("• CORS handling");
  println!("• Request/Response logging");
  println!("• Compression/Decompression");
  println!("• Rate limiting");
  println!("• Request validation");
  println!("• Response transformation");
  println!();

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

  let mut request_logging = RequestLoggingTransformer::new()
    .with_log_level(LogLevel::Info)
    .with_include_body(false);

  let response_logging = ResponseLoggingTransformer::new()
    .with_log_level(LogLevel::Info)
    .with_include_body(false);

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

  let mut validation_transformer = RequestValidationTransformer::new();

  let mut response_transform_transformer = ResponseTransformTransformer::new();

  // Create handlers
  let user_handler = Arc::new(UserApiHandler);
  let public_handler = Arc::new(PublicHandler);

  // Create test requests
  let test_requests = create_test_requests();

  println!(
    "📝 Processing {} test requests through HTTP middleware pipeline",
    test_requests.len()
  );
  println!("==================================================================");

  // Process each request through individual middleware transformers
  for (i, request) in test_requests.into_iter().enumerate() {
    println!("\n--- Request {} ---", i + 1);
    println!("Method: {}", request.method);
    println!("Path: {}", request.path());
    println!("Headers: {}", request.headers.len());

    // Test CORS
    println!("  🌐 Testing CORS...");
    let cors_stream = futures::stream::iter(vec![request.clone()]);
    let mut cors_output = cors_transformer.transform(Box::pin(cors_stream));
    let mut cors_requests = Vec::new();
    while let Some(req) = cors_output.next().await {
      cors_requests.push(req);
    }
    println!("  ✅ CORS processing completed");

    // Test Request Logging
    println!("  📝 Testing Request Logging...");
    let logging_stream = futures::stream::iter(cors_requests.clone());
    let mut logging_output = request_logging.transform(Box::pin(logging_stream));
    let mut logged_requests = Vec::new();
    while let Some(req) = logging_output.next().await {
      logged_requests.push(req);
    }
    println!("  ✅ Request logging completed");

    // Test Validation
    println!("  ✅ Testing Request Validation...");
    let validation_stream = futures::stream::iter(logged_requests.clone());
    let mut validation_output = validation_transformer.transform(Box::pin(validation_stream));
    let mut validated_requests = Vec::new();
    while let Some(req) = validation_output.next().await {
      validated_requests.push(req);
    }
    println!("  ✅ Request validation completed");

    // Test Rate Limiting
    println!("  ⏱️ Testing Rate Limiting...");
    let rate_limit_stream = futures::stream::iter(validated_requests.clone());
    let mut rate_limit_output = rate_limit_transformer.transform(Box::pin(rate_limit_stream));
    let mut rate_limited_requests: Vec<StreamWeaveHttpRequestChunk> = Vec::new();
    while let Some(req) = rate_limit_output.next().await {
      rate_limited_requests.push(req);
    }
    println!("  ✅ Rate limiting completed");

    // Process the request through the handler
    if let Some(request) = rate_limited_requests.first() {
      println!("  🎯 Processing request through handler...");
      let response = if request.path().starts_with("/api/") {
        user_handler.handle(request.clone()).await
      } else {
        public_handler.handle(request.clone()).await
      };

      println!("  ✅ Response generated");
      println!("     Status: {}", response.status);
      println!("     Headers: {}", response.headers.len());
      println!("     Body size: {} bytes", response.body.len());

      // Test Response Logging
      println!("  📝 Testing Response Logging...");
      response_logging.log_response(&response, Some("req-123"));
      println!("  ✅ Response logging completed");

      // Test Compression
      println!("  🗜️ Testing Compression...");
      let compression_stream = futures::stream::iter(vec![response.clone()]);
      let mut compression_output = compression_transformer.transform(Box::pin(compression_stream));
      let mut compressed_responses = Vec::new();
      while let Some(resp) = compression_output.next().await {
        compressed_responses.push(resp);
      }
      if let Some(compressed_response) = compressed_responses.first() {
        println!("  ✅ Compression completed");
        if let Some(encoding) = compressed_response.headers.get("content-encoding") {
          println!(
            "     Content-Encoding: {}",
            encoding.to_str().unwrap_or("invalid")
          );
        }
      }

      // Test Response Transformation
      println!("  🔄 Testing Response Transformation...");
      let transform_stream = futures::stream::iter(compressed_responses);
      let mut transform_output =
        response_transform_transformer.transform(Box::pin(transform_stream));
      let mut transformed_responses = Vec::new();
      while let Some(resp) = transform_output.next().await {
        transformed_responses.push(resp);
      }
      if let Some(final_response) = transformed_responses.first() {
        println!("  ✅ Response transformation completed");
        println!("     Final status: {}", final_response.status);
        println!("     Final headers: {}", final_response.headers.len());
        println!("     Final body size: {} bytes", final_response.body.len());
      }
    }
  }

  println!("\n🎉 HTTP Middleware example completed successfully!");
  println!("\nKey Features Demonstrated:");
  println!("• 🌐 CORS: Cross-Origin Resource Sharing handling");
  println!("• 📝 Logging: Request and response logging with structured data");
  println!("• 🗜️  Compression: Automatic response compression (Gzip, Deflate, Brotli)");
  println!("• ⏱️  Rate Limiting: Token bucket rate limiting per API key");
  println!("• ✅ Validation: Request validation (headers, body size, content type)");
  println!("• 🔄 Response Transformation: Security headers, caching, JSON minification");
  println!("• 🚦 Pipeline: All middleware working together in a processing pipeline");
  println!(
    "• 🆕 HTTP Response Builder: Structured response generation with security headers and CORS"
  );

  Ok(())
}
