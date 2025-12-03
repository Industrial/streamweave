//! HTTP Request Handlers
//!
//! This module implements various handler types that process HTTP requests
//! within the graph. Each handler demonstrates different patterns and use cases.

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use axum::http::StatusCode;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use serde_json::json;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use streamweave::http_server::{HttpRequest, HttpResponse};
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use streamweave::message::Message;

/// Handles REST API requests
///
/// This handler processes REST API requests, demonstrating:
/// - GET requests for resource retrieval
/// - POST requests for resource creation
/// - Path parameter extraction
/// - Query parameter handling
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub fn handle_rest_request(msg: Message<HttpRequest>) -> Message<HttpResponse> {
  let request = msg.payload();
  let request_id = request.request_id.clone();

  // Extract path segments
  let path = &request.path;
  let response = if path.starts_with("/api/rest/users") {
    match request.method {
      streamweave::http_server::HttpMethod::Get => {
        // GET /api/rest/users - List users
        let users = json!([
          {"id": 1, "name": "Alice", "email": "alice@example.com"},
          {"id": 2, "name": "Bob", "email": "bob@example.com"},
          {"id": 3, "name": "Charlie", "email": "charlie@example.com"},
        ]);
        HttpResponse::json_with_request_id(StatusCode::OK, &users, request_id).unwrap()
      }
      streamweave::http_server::HttpMethod::Post => {
        // POST /api/rest/users - Create user
        let body_str = request
          .body
          .as_ref()
          .map(|b| String::from_utf8_lossy(b).to_string())
          .unwrap_or_default();
        let response_data = json!({
          "message": "User created",
          "request_body": body_str,
          "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        HttpResponse::json_with_request_id(StatusCode::CREATED, &response_data, request_id).unwrap()
      }
      _ => HttpResponse::error_with_request_id(
        StatusCode::METHOD_NOT_ALLOWED,
        "Method not allowed for this endpoint",
        request_id,
      ),
    }
  } else if path.starts_with("/api/rest/posts") {
    // GET /api/rest/posts - List posts
    let posts = json!([
      {"id": 1, "title": "First Post", "author": "Alice"},
      {"id": 2, "title": "Second Post", "author": "Bob"},
    ]);
    HttpResponse::json_with_request_id(StatusCode::OK, &posts, request_id).unwrap()
  } else {
    // Unknown REST endpoint
    HttpResponse::error_with_request_id(
      StatusCode::NOT_FOUND,
      &format!("REST endpoint not found: {}", path),
      request_id,
    )
  };

  Message::new(response, msg.id().clone())
}

/// Creates a GraphQL-style handler
///
/// This handler processes GraphQL-style queries, demonstrating:
/// - POST requests with JSON body
/// - Query parsing and execution
/// - Response formatting
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub fn handle_graphql_request(msg: Message<HttpRequest>) -> Message<HttpResponse> {
  let request = msg.payload();
  let request_id = request.request_id.clone();

  // Parse GraphQL query from request body
  let body_str = request
    .body
    .as_ref()
    .map(|b| String::from_utf8_lossy(b).to_string())
    .unwrap_or_default();
  let response_data = if let Ok(query_json) = serde_json::from_str::<serde_json::Value>(&body_str) {
    // Extract query and variables
    let query = query_json
      .get("query")
      .and_then(|v| v.as_str())
      .unwrap_or("");
    let variables = query_json.get("variables").cloned().unwrap_or(json!({}));

    // Simple GraphQL-style execution
    let data = if query.contains("users") {
      json!({
        "users": [
          {"id": "1", "name": "Alice", "email": "alice@example.com"},
          {"id": "2", "name": "Bob", "email": "bob@example.com"},
        ]
      })
    } else if query.contains("posts") {
      json!({
        "posts": [
          {"id": "1", "title": "First Post", "author": "Alice"},
          {"id": "2", "title": "Second Post", "author": "Bob"},
        ]
      })
    } else {
      json!({"message": "Unknown query"})
    };

    json!({
      "data": data,
      "query": query,
      "variables": variables,
    })
  } else {
    json!({
      "errors": [{"message": "Invalid GraphQL query format"}],
      "data": null,
    })
  };

  let response =
    HttpResponse::json_with_request_id(StatusCode::OK, &response_data, request_id).unwrap();
  Message::new(response, msg.id().clone())
}

/// Creates an RPC handler
///
/// This handler processes RPC-style requests, demonstrating:
/// - Method-based routing
/// - JSON-RPC style request/response
/// - Error handling
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub fn handle_rpc_request(msg: Message<HttpRequest>) -> Message<HttpResponse> {
  let request = msg.payload();
  let request_id = request.request_id.clone();

  // Extract RPC method from path
  let path = &request.path;
  let method = path
    .strip_prefix("/api/rpc/")
    .unwrap_or("unknown")
    .to_string();

  // Parse RPC request body
  let body_str = request
    .body
    .as_ref()
    .map(|b| String::from_utf8_lossy(b).to_string())
    .unwrap_or_default();
  let params = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body_str) {
    json.get("params").cloned().unwrap_or(json!({}))
  } else {
    json!({})
  };

  // Execute RPC method
  let result = match method.as_str() {
    "echo" => {
      // Echo RPC method - returns the input
      json!({
        "result": params,
        "method": "echo",
      })
    }
    "add" => {
      // Add RPC method - adds two numbers
      let a = params.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
      let b = params.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
      json!({
        "result": a + b,
        "method": "add",
      })
    }
    "multiply" => {
      // Multiply RPC method - multiplies two numbers
      let a = params.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
      let b = params.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
      json!({
        "result": a * b,
        "method": "multiply",
      })
    }
    _ => {
      // Unknown RPC method
      return Message::new(
        HttpResponse::error_with_request_id(
          StatusCode::NOT_FOUND,
          &format!("RPC method not found: {}", method),
          request_id,
        ),
        msg.id().clone(),
      );
    }
  };

  let response_data = json!({
    "jsonrpc": "2.0",
    "result": result,
    "id": request_id,
  });

  let response =
    HttpResponse::json_with_request_id(StatusCode::OK, &response_data, request_id).unwrap();
  Message::new(response, msg.id().clone())
}

/// Creates a static file handler
///
/// This handler serves static content, demonstrating:
/// - File path extraction
/// - Content type detection
/// - Simple file serving
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub fn handle_static_request(msg: Message<HttpRequest>) -> Message<HttpResponse> {
  let request = msg.payload();
  let request_id = request.request_id.clone();

  // Extract file path
  let path = request
    .path
    .strip_prefix("/static/")
    .unwrap_or("index.html");

  // Simple static content mapping
  let (content, _content_type) = match path {
    "index.html" => (
      r#"<!DOCTYPE html>
<html>
<head>
    <title>StreamWeave HTTP Graph Server</title>
</head>
<body>
    <h1>Welcome to StreamWeave HTTP Graph Server!</h1>
    <p>This is a static file served through the graph.</p>
</body>
</html>"#,
      "text/html",
    ),
    "style.css" => (
      r#"body { font-family: Arial, sans-serif; margin: 40px; }
h1 { color: #333; }"#,
      "text/css",
    ),
    "app.js" => (
      r#"console.log('Hello from StreamWeave!');"#,
      "application/javascript",
    ),
    _ => {
      // File not found
      return Message::new(
        HttpResponse::error_with_request_id(
          StatusCode::NOT_FOUND,
          &format!("Static file not found: {}", path),
          request_id,
        ),
        msg.id().clone(),
      );
    }
  };

  let response = HttpResponse::text_with_request_id(StatusCode::OK, content, request_id);
  Message::new(response, msg.id().clone())
}

/// Creates a fan-out handler
///
/// This handler demonstrates fan-out patterns where one request
/// is processed by multiple handlers and results are combined.
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub fn handle_fanout_request(msg: Message<HttpRequest>) -> Message<HttpResponse> {
  let request = msg.payload();
  let request_id = request.request_id.clone();

  // Simulate fan-out processing
  // In a real scenario, this would send the request to multiple handlers
  // and merge the results
  let response_data = json!({
    "message": "Fan-out processing example",
    "original_path": request.path,
    "processed_by": [
      "handler_1",
      "handler_2",
      "handler_3",
    ],
    "results": [
      {"handler": "handler_1", "result": "processed"},
      {"handler": "handler_2", "result": "processed"},
      {"handler": "handler_3", "result": "processed"},
    ],
    "timestamp": chrono::Utc::now().to_rfc3339(),
  });

  let response =
    HttpResponse::json_with_request_id(StatusCode::OK, &response_data, request_id).unwrap();
  Message::new(response, msg.id().clone())
}

/// Creates a default/404 handler
///
/// This handler processes requests that don't match any route pattern.
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub fn handle_default_request(msg: Message<HttpRequest>) -> Message<HttpResponse> {
  let request = msg.payload();
  let request_id = request.request_id.clone();

  // Health check endpoint
  if request.path == "/api/health" {
    let response_data = json!({
      "status": "healthy",
      "timestamp": chrono::Utc::now().to_rfc3339(),
      "server": "StreamWeave HTTP Graph Server",
    });
    let response =
      HttpResponse::json_with_request_id(StatusCode::OK, &response_data, request_id).unwrap();
    return Message::new(response, msg.id().clone());
  }

  // 404 Not Found
  let response_data = json!({
    "error": "Not Found",
    "message": format!("The requested path '{}' was not found on this server.", request.path),
    "path": request.path,
    "method": format!("{:?}", request.method),
  });

  let response =
    HttpResponse::json_with_request_id(StatusCode::NOT_FOUND, &response_data, request_id).unwrap();
  Message::new(response, msg.id().clone())
}
