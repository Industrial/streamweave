use http::Method;
use std::collections::HashMap;
use std::sync::Arc;
use streamweave::{
  consumers::vec::vec_consumer::VecConsumer,
  http::{
    http_handler::HttpHandler, http_request_chunk::StreamWeaveHttpRequestChunk,
    http_response::StreamWeaveHttpResponse, route_pattern::RoutePattern,
  },
  pipeline::PipelineBuilder,
  producers::vec::vec_producer::VecProducer,
  transformer::{Transformer, TransformerConfig},
  transformers::{
    http_response_builder::{
      builder_utils::{ErrorResponseBuilder, HtmlResponseBuilder, JsonResponseBuilder, responses},
      response_data::ResponseData,
    },
    http_router::transformer::HttpRouterTransformer,
  },
};

/// Example HTTP handler that returns a simple response
struct HelloHandler;

#[async_trait::async_trait]
impl HttpHandler for HelloHandler {
  async fn handle(&self, request: StreamWeaveHttpRequestChunk) -> StreamWeaveHttpResponse {
    let path = request.path();
    let method = &request.method;

    // Use HTTP Response Builder for a more structured response
    let response_data = HtmlResponseBuilder::with_status(http::StatusCode::OK)
      .content(&format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>StreamWeave Router - Hello</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; background: #f5f5f5; }}
        .container {{ background: white; padding: 30px; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }}
        .method {{ color: #0066cc; font-weight: bold; }}
        .path {{ color: #009900; font-weight: bold; }}
        pre {{ background: #f8f8f8; padding: 15px; border-radius: 5px; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 Hello from StreamWeave Router!</h1>
        <p>Request processed successfully:</p>
        <ul>
            <li><strong>Method:</strong> <span class="method">{}</span></li>
            <li><strong>Path:</strong> <span class="path">{}</span></li>
            <li><strong>Path Params:</strong> <pre>{:?}</pre></li>
            <li><strong>Query Params:</strong> <pre>{:?}</pre></li>
        </ul>
        <p><em>Response generated using HTTP Response Builder</em></p>
    </div>
</body>
</html>"#,
        method, path, request.path_params, request.query_params
      ))
      .header("x-powered-by", "StreamWeave HTTP Response Builder")
      .build();

    // Convert ResponseData to StreamWeaveHttpResponse
    match response_data {
      ResponseData::Success {
        status,
        headers,
        body,
      } => StreamWeaveHttpResponse::new(status, headers, body),
      ResponseData::Error { status, message } => {
        StreamWeaveHttpResponse::new(status, http::HeaderMap::new(), message.into())
      }
    }
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

    // Use JSON Response Builder for structured API response
    let response_data = JsonResponseBuilder::with_status(http::StatusCode::OK)
      .string_field("user_id", &user_id)
      .string_field("name", &format!("User {}", user_id))
      .string_field("email", &format!("user{}@example.com", user_id))
      .string_field("profile_url", &format!("/users/{}/profile", user_id))
      .string_field("generated_by", "StreamWeave HTTP Response Builder")
      .header("x-api-version", "v1")
      .header("x-powered-by", "StreamWeave")
      .build();

    // Convert ResponseData to StreamWeaveHttpResponse
    match response_data {
      ResponseData::Success {
        status,
        headers,
        body,
      } => StreamWeaveHttpResponse::new(status, headers, body),
      ResponseData::Error { status, message } => {
        StreamWeaveHttpResponse::new(status, http::HeaderMap::new(), message.into())
      }
    }
  }
}

/// Example handler for API routes
struct ApiHandler;

#[async_trait::async_trait]
impl HttpHandler for ApiHandler {
  async fn handle(&self, request: StreamWeaveHttpRequestChunk) -> StreamWeaveHttpResponse {
    let path = request.path();
    let method = &request.method;

    // Use convenience functions from the responses module
    let response_data = responses::ok_json(serde_json::json!({
      "message": "API response",
      "method": method.as_str(),
      "path": path,
      "timestamp": chrono::Utc::now().to_rfc3339(),
      "features": [
        "HTTP Response Builder",
        "Security Headers",
        "CORS Support",
        "JSON Response Builder"
      ],
      "powered_by": "StreamWeave"
    }));

    // Convert ResponseData to StreamWeaveHttpResponse
    match response_data {
      ResponseData::Success {
        status,
        headers,
        body,
      } => StreamWeaveHttpResponse::new(status, headers, body),
      ResponseData::Error { status, message } => {
        StreamWeaveHttpResponse::new(status, http::HeaderMap::new(), message.into())
      }
    }
  }
}

/// Example fallback handler for 404 responses
struct NotFoundHandler;

#[async_trait::async_trait]
impl HttpHandler for NotFoundHandler {
  async fn handle(&self, request: StreamWeaveHttpRequestChunk) -> StreamWeaveHttpResponse {
    // Use ErrorResponseBuilder for proper error responses
    let response_data = ErrorResponseBuilder::not_found(&format!(
      "No route found for {} {}",
      request.method,
      request.path()
    ))
    .header("x-powered-by", "StreamWeave HTTP Response Builder")
    .build();

    // Convert ResponseData to StreamWeaveHttpResponse
    match response_data {
      ResponseData::Success {
        status,
        headers,
        body,
      } => StreamWeaveHttpResponse::new(status, headers, body),
      ResponseData::Error { status, message } => {
        StreamWeaveHttpResponse::new(status, http::HeaderMap::new(), message.into())
      }
    }
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Initialize logging
  tracing_subscriber::fmt::init();

  println!("🚀 StreamWeave HTTP Router Transformer Example");
  println!("===============================================");
  println!("Now featuring full StreamWeave pipeline architecture with HTTP Response Builder!");
  println!();

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
    .transformer(router)
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
        if let Some(cors) = response.headers.get("access-control-allow-origin") {
          println!("CORS Origin: {}", cors.to_str().unwrap_or("invalid"));
        }

        // Show a preview of the response body
        let body_preview = String::from_utf8_lossy(&response.body);
        let preview = if body_preview.len() > 200 {
          format!("{}...", &body_preview[..200])
        } else {
          body_preview.to_string()
        };
        println!("Body preview: {}", preview);
      }
    }
    Err(e) => {
      println!("❌ Pipeline error: {}", e);
    }
  }

  println!("\n🎉 HTTP Router Transformer example completed successfully!");
  println!("\nKey Features Demonstrated:");
  println!("• 🛣️  HTTP Router: Request routing with path parameters and fallback handling");
  println!("• 🆕 HTTP Response Builder: Structured response generation with security headers");
  println!("• 🔗 StreamWeave Architecture: Complete request-to-response pipeline in one stream");
  println!("• Route pattern matching with path parameters");
  println!("• Query parameter parsing");
  println!("• Multiple HTTP methods support");
  println!("• Fallback handler for 404 responses");
  println!("• HTTP Response Builder integration:");
  println!("  - HtmlResponseBuilder for rich HTML responses");
  println!("  - JsonResponseBuilder for structured API responses");
  println!("  - ErrorResponseBuilder for proper error responses");
  println!("  - Convenience functions for common response types");
  println!("  - Automatic security headers");
  println!("  - CORS support");
  println!("  - Content-Type header management");
  println!("• Pipeline Processing: All requests processed through a single continuous stream");

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
