use http::Method;
use std::collections::HashMap;
use std::sync::Arc;
use futures::StreamExt;
use streamweave::{
  http::{
    http_handler::HttpHandler, http_request_chunk::StreamWeaveHttpRequestChunk,
    http_response::StreamWeaveHttpResponse, route_pattern::RoutePattern,
  },
  transformers::http_router::transformer::HttpRouterTransformer,
  transformer::{Transformer, TransformerConfig},
};

/// Example HTTP handler that returns a simple response
struct HelloHandler;

#[async_trait::async_trait]
impl HttpHandler for HelloHandler {
  async fn handle(&self, request: StreamWeaveHttpRequestChunk) -> StreamWeaveHttpResponse {
    let path = request.path();
    let method = &request.method;

    let body = format!(
      "Hello from {} {}! Path params: {:?}, Query params: {:?}",
      method, path, request.path_params, request.query_params
    );

    StreamWeaveHttpResponse::ok(body.into())
  }
}

/// Example handler for user-specific routes
struct UserHandler;

#[async_trait::async_trait]
impl HttpHandler for UserHandler {
  async fn handle(&self, request: StreamWeaveHttpRequestChunk) -> StreamWeaveHttpResponse {
    let user_id = request
      .path_param("id")
      .unwrap_or(&"unknown".to_string())
      .clone();
    let body = format!("User profile for ID: {}", user_id);

    StreamWeaveHttpResponse::ok(body.into())
  }
}

/// Example handler for API routes
struct ApiHandler;

#[async_trait::async_trait]
impl HttpHandler for ApiHandler {
  async fn handle(&self, request: StreamWeaveHttpRequestChunk) -> StreamWeaveHttpResponse {
    let path = request.path();
    let method = &request.method;

    let body = format!(
      r#"{{"message": "API response", "method": "{}", "path": "{}", "timestamp": "{}"}}"#,
      method,
      path,
      chrono::Utc::now().to_rfc3339()
    );

    StreamWeaveHttpResponse::ok(body.into()).with_content_type("application/json")
  }
}

/// Example fallback handler for 404 responses
struct NotFoundHandler;

#[async_trait::async_trait]
impl HttpHandler for NotFoundHandler {
  async fn handle(&self, request: StreamWeaveHttpRequestChunk) -> StreamWeaveHttpResponse {
    let body = format!(
      r#"{{"error": "Not Found", "message": "No route found for {} {}", "path": "{}"}}"#,
      request.method,
      request.path(),
      request.path()
    );

    StreamWeaveHttpResponse::not_found(body.into()).with_content_type("application/json")
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Initialize logging
  tracing_subscriber::fmt::init();

  println!("🚀 StreamWeave HTTP Router Transformer Example");
  println!("===============================================");

  // Create route patterns
  let hello_route = RoutePattern::new(Method::GET, "/hello")?;
  let user_route = RoutePattern::new(Method::GET, "/users/{id}")?;
  let api_route = RoutePattern::new(Method::GET, "/api/*")?;
  let post_route = RoutePattern::new(Method::POST, "/api/data")?;

  // Create handlers
  let hello_handler = Arc::new(HelloHandler);
  let user_handler = Arc::new(UserHandler);
  let api_handler = Arc::new(ApiHandler);
  let not_found_handler = Arc::new(NotFoundHandler);

  // Build the HTTP router transformer
  let mut router = HttpRouterTransformer::new()
    .add_route(hello_route, "hello".to_string(), hello_handler)?
    .add_route(user_route, "user".to_string(), user_handler)?
    .add_route(api_route, "api".to_string(), api_handler.clone())?
    .add_route(post_route, "api_post".to_string(), api_handler)?
    .with_fallback_handler(not_found_handler);
  
  router.set_config(TransformerConfig::default().with_name("http_router".to_string()));

  // Create test requests
  let test_requests = vec![
    create_test_request(Method::GET, "/hello", HashMap::new()),
    create_test_request(Method::GET, "/users/123", HashMap::new()),
    create_test_request(Method::GET, "/api/status", HashMap::new()),
    create_test_request(Method::POST, "/api/data", HashMap::new()),
    create_test_request(Method::GET, "/nonexistent", HashMap::new()),
    create_test_request(Method::GET, "/users/456?include=profile", HashMap::new()),
  ];

  println!("\n📝 Testing HTTP Router Transformer");
  println!("===================================");

  // Process each test request using the transformer
  for (i, request) in test_requests.into_iter().enumerate() {
    println!("\n--- Test {} ---", i + 1);
    println!("Request: {} {}", request.method, request.path());

    // Create a simple input stream with just this request
    let input_stream = futures::stream::iter(vec![request]);
    let mut output_stream = router.transform(Box::pin(input_stream));

    // Process the response
    if let Some(response) = output_stream.next().await {
      println!(
        "Response: {} - {}",
        response.status,
        String::from_utf8_lossy(&response.body)
      );
    } else {
      println!("❌ No response generated");
    }
  }

  println!("\n🎉 HTTP Router Transformer example completed successfully!");
  println!("\nKey Features Demonstrated:");
  println!("• Route pattern matching with path parameters");
  println!("• Query parameter parsing");
  println!("• Multiple HTTP methods support");
  println!("• Fallback handler for 404 responses");
  println!("• JSON response formatting");
  println!("• Content-Type header management");

  Ok(())
}

/// Helper function to create test requests
fn create_test_request(
  method: Method,
  path: &str,
  _path_params: HashMap<String, String>,
) -> StreamWeaveHttpRequestChunk {
  use http::Uri;
  use std::net::{IpAddr, Ipv4Addr, SocketAddr};

  let uri: Uri = format!("http://localhost:3000{}", path).parse().unwrap();

  StreamWeaveHttpRequestChunk::new(
    method,
    uri,
    http::HeaderMap::new(),
    bytes::Bytes::new(),
    streamweave::http::connection_info::ConnectionInfo::new(
      SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
      SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 3000),
      http::Version::HTTP_11,
    ),
    true,
  )
}