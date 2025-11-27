//! # HTTP Server Integration Pipeline Examples
//!
//! This module demonstrates how to use StreamWeave's HTTP server capabilities
//! to build REST microservices with Axum integration.

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use axum::{
  Router,
  http::StatusCode,
  routing::{delete, get, post, put},
};
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use chrono::Utc;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use std::net::SocketAddr;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use streamweave::http_server::{
  ContentType, HttpRequest, HttpResponse, create_custom_error, create_pipeline_handler,
  create_simple_handler, is_development_mode, map_to_http_error,
};
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use streamweave::transformers::map::map_transformer::MapTransformer;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
use tokio::net::TcpListener;

/// Runs the HTTP server with multiple example endpoints.
///
/// This demonstrates:
/// - Task 16.1: HTTP server types (HttpRequest, HttpResponse, HttpMethod, ContentType)
/// - Task 16.2: HTTP request producer
/// - Task 16.3: HTTP response consumer
/// - Task 16.4: Axum route handler integration
/// - Task 16.5: Streaming request bodies
// Task 16.5: Handler for streaming request bodies
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
async fn handle_stream_upload(
  request: axum::extract::Request,
) -> axum::response::Response<axum::body::Body> {
  use streamweave::consumers::vec::vec_consumer::VecConsumer;
  use streamweave::http_server::{HttpRequestProducer, HttpRequestProducerConfig};
  use streamweave::pipeline::PipelineBuilder;
  use streamweave::transformers::map::map_transformer::MapTransformer;

  // Create producer with streaming enabled
  let producer = HttpRequestProducer::from_axum_request(
    request,
    HttpRequestProducerConfig::default()
      .with_stream_body(true)
      .with_chunk_size(64 * 1024), // 64KB chunks
  )
  .await;

  // Create a transformer that processes chunks and tracks progress
  let transformer = MapTransformer::new(|req: HttpRequest| {
    if req.is_body_chunk() {
      // This is a body chunk - process it
      let offset = req.get_chunk_offset().unwrap_or(0);
      let chunk_size = req.get_chunk_size().unwrap_or(0);
      let body_size = req.body.as_ref().map(|b| b.len()).unwrap_or(0);

      // Create a response item with progress info
      serde_json::json!({
        "type": "chunk",
        "offset": offset,
        "chunk_size": chunk_size,
        "body_size": body_size,
        "content_type": req.content_type.as_ref().map(|ct| format!("{:?}", ct)),
      })
    } else {
      // This is the initial request metadata
      serde_json::json!({
        "type": "metadata",
        "method": format!("{:?}", req.method),
        "path": req.path,
        "content_type": req.content_type.as_ref().map(|ct| format!("{:?}", ct)),
        "has_body": req.body.is_some(),
      })
    }
  });

  // Create consumer to collect results
  let consumer = VecConsumer::new();

  // Build and run pipeline
  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .consumer(consumer);

  match pipeline.run().await {
    Ok((_, consumer)) => {
      let results = consumer.into_vec();
      let total_chunks = results
        .iter()
        .filter(|r| {
          r.get("type")
            .and_then(|t| t.as_str())
            .map(|t| t == "chunk")
            .unwrap_or(false)
        })
        .count();

      HttpResponse::json(
        StatusCode::OK,
        &serde_json::json!({
          "message": "Streaming upload processed successfully",
          "total_chunks": total_chunks,
          "items": results,
        }),
      )
      .unwrap()
      .to_axum_response()
    }
    Err(e) => HttpResponse::error(
      StatusCode::INTERNAL_SERVER_ERROR,
      &format!("Pipeline error: {:?}", e),
    )
    .to_axum_response(),
  }
}

// Task 16.6: Handler for streaming response bodies
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
async fn handle_stream_download(
  _request: axum::extract::Request,
) -> axum::response::Response<axum::body::Body> {
  use async_stream::stream;
  use streamweave::http_server::consumer::HttpResponseConsumer;

  // Create a stream of HttpResponse items that will be streamed to the client
  // In a real scenario, this would come from a database query, file read, or pipeline
  let response_stream = stream! {
    for i in 1..=5 {
      let item = serde_json::json!({
        "id": i,
        "data": format!("chunk{}", i),
        "timestamp": chrono::Utc::now().to_rfc3339(),
      });

      // Create HttpResponse for each item
      if let Ok(response) = HttpResponse::json(StatusCode::OK, &item) {
        yield response;
      } else {
        // Fallback to binary if JSON serialization fails
        let body = serde_json::to_vec(&item).unwrap();
        yield HttpResponse::binary(StatusCode::OK, body);
      }

      // Small delay to demonstrate streaming
      tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
  };

  // Create streaming response using the consumer
  let consumer = HttpResponseConsumer::new();
  consumer.create_streaming_response(response_stream).await
}

// Helper function to create async handler from simple handler
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
async fn echo_handler(
  request: axum::extract::Request,
) -> axum::response::Response<axum::body::Body> {
  let handler = create_simple_handler(|req: HttpRequest| {
    // Task 16.1: Demonstrate HttpRequest usage
    let method = req.method;
    let path = req.path.clone();
    let query_params = req.query_params.clone();
    let headers_count = req.headers.len();

    // Task 16.1: Demonstrate HttpResponse creation
    HttpResponse::json(
      StatusCode::OK,
      &serde_json::json!({
        "method": format!("{:?}", method),
        "path": path,
        "query_params": query_params,
        "headers_count": headers_count,
        "message": "Echo endpoint - demonstrates HttpRequest and HttpResponse types"
      }),
    )
    .unwrap()
  });
  handler(request).await
}

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
  // Build the router with various endpoints demonstrating different capabilities
  let app = Router::new()
    // Task 16.1 & 16.4: Simple handler demonstrating HTTP types and basic integration
    .route("/api/echo", get(echo_handler))
    // Task 16.1: Demonstrate ContentType handling
    .route(
      "/api/content-type",
      post(|request: axum::extract::Request| async move {
        let handler = create_simple_handler(|req: HttpRequest| {
          // Task 16.1: Check content type
          let content_type = req
            .content_type
            .map(|ct| format!("{:?}", ct))
            .unwrap_or_else(|| "unknown".to_string());

          let body_size = req.body.as_ref().map(|b| b.len()).unwrap_or(0);

          HttpResponse::json(
            StatusCode::OK,
            &serde_json::json!({
              "content_type": content_type,
              "body_size": body_size,
              "message": "Content type detection example"
            }),
          )
          .unwrap()
        });
        handler(request).await
      }),
    )
    // Task 16.2: Demonstrate HTTP request producer with query parameters
    .route(
      "/api/query",
      get(|request: axum::extract::Request| async move {
        let handler = create_simple_handler(|req: HttpRequest| {
          // Task 16.2: Extract query parameters using HttpRequestProducer
          let name = req.get_query_param("name").cloned();
          let age = req
            .get_query_param("age")
            .and_then(|s| s.parse::<u32>().ok());

          HttpResponse::json(
            StatusCode::OK,
            &serde_json::json!({
              "name": name,
              "age": age,
              "all_params": req.query_params,
              "message": "Query parameter extraction example"
            }),
          )
          .unwrap()
        });
        handler(request).await
      }),
    )
    // Task 16.2: Demonstrate HTTP request producer with body extraction
    .route(
      "/api/body",
      post(|request: axum::extract::Request| async move {
        let handler = create_simple_handler(|req: HttpRequest| {
          // Task 16.2: Extract request body
          let body_text = req
            .body
            .as_ref()
            .and_then(|b| std::str::from_utf8(b).ok())
            .map(|s| s.to_string());

          HttpResponse::json(
            StatusCode::OK,
            &serde_json::json!({
              "body_received": body_text.is_some(),
              "body_length": req.body.as_ref().map(|b| b.len()).unwrap_or(0),
              "body_preview": body_text.as_ref().map(|s| {
                if s.len() > 100 {
                  format!("{}...", &s[..100])
                } else {
                  s.clone()
                }
              }),
              "message": "Request body extraction example"
            }),
          )
          .unwrap()
        });
        handler(request).await
      }),
    )
    // Task 16.3 & 16.4: Demonstrate HTTP response consumer with pipeline
    .route(
      "/api/process",
      post(|request: axum::extract::Request| async move {
        let handler = create_pipeline_handler(|| {
          // Task 16.3: Transform HttpRequest to HttpResponse using pipeline
          MapTransformer::new(|req: HttpRequest| {
            // Process the request
            let processed_data = serde_json::json!({
              "original_path": req.path,
              "method": format!("{:?}", req.method),
              "processed_at": Utc::now().to_rfc3339(),
              "body_size": req.body.as_ref().map(|b| b.len()).unwrap_or(0),
            });

            // Task 16.3: Create HttpResponse with custom status and headers
            let mut response = HttpResponse::json(StatusCode::OK, &processed_data).unwrap();
            response.add_header(
              "x-processed-by",
              axum::http::HeaderValue::from_static("streamweave"),
            );
            response
          })
        });
        handler(request).await
      }),
    )
    // Task 16.1 & 16.4: Demonstrate different HTTP methods
    .route(
      "/api/methods",
      get(|request: axum::extract::Request| async move {
        let handler = create_simple_handler(|req: HttpRequest| {
          HttpResponse::json(
            StatusCode::OK,
            &serde_json::json!({
              "method": format!("{:?}", req.method),
              "message": "GET request example"
            }),
          )
          .unwrap()
        });
        handler(request).await
      }),
    )
    .route(
      "/api/methods",
      put(|request: axum::extract::Request| async move {
        let handler = create_simple_handler(|req: HttpRequest| {
          HttpResponse::json(
            StatusCode::OK,
            &serde_json::json!({
              "method": format!("{:?}", req.method),
              "message": "PUT request example"
            }),
          )
          .unwrap()
        });
        handler(request).await
      }),
    )
    .route(
      "/api/methods",
      delete(|request: axum::extract::Request| async move {
        let handler = create_simple_handler(|req: HttpRequest| {
          HttpResponse::json(
            StatusCode::OK,
            &serde_json::json!({
              "method": format!("{:?}", req.method),
              "message": "DELETE request example"
            }),
          )
          .unwrap()
        });
        handler(request).await
      }),
    )
    // Task 16.8: Demonstrate comprehensive error handling
    .route(
      "/api/error",
      get(|request: axum::extract::Request| async move {
        let handler = create_simple_handler(|_req: HttpRequest| {
          // Task 16.8: Create error response using error handling utilities
          HttpResponse::error(StatusCode::BAD_REQUEST, "This is an example error response")
        });
        handler(request).await
      }),
    )
    // Task 16.8: Demonstrate different error types
    .route(
      "/api/error/validation",
      get(|request: axum::extract::Request| async move {
        use streamweave::error::{ComponentInfo, ErrorContext, StreamError};
        use streamweave::http_server::is_development_mode;

        let handler = create_simple_handler(|_req: HttpRequest| {
          // Create a validation error
          let stream_error = StreamError::<HttpRequest>::new(
            Box::new(std::io::Error::other(
              "Invalid input: missing required field 'name'",
            )),
            ErrorContext {
              timestamp: Utc::now(),
              item: None,
              component_name: "validator".to_string(),
              component_type: "ValidatorTransformer".to_string(),
            },
            ComponentInfo {
              name: "validator".to_string(),
              type_name: "ValidatorTransformer".to_string(),
            },
          );

          // Map to HTTP error response
          let error_response = map_to_http_error(&stream_error, is_development_mode());
          HttpResponse::json(StatusCode::BAD_REQUEST, &error_response).unwrap()
        });
        handler(request).await
      }),
    )
    .route(
      "/api/error/not-found",
      get(|request: axum::extract::Request| async move {
        use streamweave::error::{ComponentInfo, ErrorContext, StreamError};
        use streamweave::http_server::is_development_mode;

        let handler = create_simple_handler(|_req: HttpRequest| {
          // Create a not found error
          let stream_error = StreamError::<HttpRequest>::new(
            Box::new(std::io::Error::other(
              "Resource not found: user with id 123",
            )),
            ErrorContext {
              timestamp: Utc::now(),
              item: None,
              component_name: "database".to_string(),
              component_type: "DatabaseProducer".to_string(),
            },
            ComponentInfo {
              name: "database".to_string(),
              type_name: "DatabaseProducer".to_string(),
            },
          );

          // Map to HTTP error response
          let error_response = map_to_http_error(&stream_error, is_development_mode());
          HttpResponse::json(StatusCode::NOT_FOUND, &error_response).unwrap()
        });
        handler(request).await
      }),
    )
    .route(
      "/api/error/custom",
      get(|request: axum::extract::Request| async move {
        let handler = create_simple_handler(|_req: HttpRequest| {
          // Create a custom error response
          let error_response = create_custom_error(
            StatusCode::CONFLICT,
            "Resource already exists",
            Some("ConflictError".to_string()),
            is_development_mode(),
          );
          HttpResponse::json(StatusCode::CONFLICT, &error_response).unwrap()
        });
        handler(request).await
      }),
    )
    // Task 16.1: Demonstrate different content types
    .route(
      "/api/content-types",
      post(|request: axum::extract::Request| async move {
        let handler = create_simple_handler(|req: HttpRequest| {
          // Task 16.1: Check and respond with different content types
          match req.content_type {
            Some(ContentType::Json) => HttpResponse::json(
              StatusCode::OK,
              &serde_json::json!({
                "detected": "JSON",
                "message": "Received JSON content"
              }),
            )
            .unwrap(),
            Some(ContentType::Text) => HttpResponse::text(StatusCode::OK, "Received text content"),
            Some(ContentType::Binary) => {
              HttpResponse::binary(StatusCode::OK, b"Received binary content".to_vec())
            }
            _ => HttpResponse::text(StatusCode::OK, "Received unknown or no content type"),
          }
        });
        handler(request).await
      }),
    )
    // Task 16.5: Demonstrate streaming request bodies
    .route("/api/stream-upload", post(handle_stream_upload))
    // Task 16.6: Demonstrate streaming response bodies
    .route("/api/stream-download", get(handle_stream_download));

  // Task 16.7: Apply middleware (CORS and logging)
  let app = app
    .layer(streamweave::http_server::cors_layer())
    .layer(streamweave::http_server::logging_layer());

  // Start the server
  let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
  let listener = TcpListener::bind(addr).await?;
  println!("✅ Server listening on http://{}", addr);
  println!();
  println!("Middleware applied:");
  println!("  ✅ CORS (allows all origins, methods, headers)");
  println!("  ✅ Logging (tracing-based request/response logging)");
  println!();
  println!("Available endpoints:");
  println!("  GET  /api/echo              - Echo request information");
  println!("  POST /api/content-type      - Check content type");
  println!("  GET  /api/query?name=test   - Extract query parameters");
  println!("  POST /api/body              - Extract request body");
  println!("  POST /api/process            - Process request through pipeline");
  println!("  GET  /api/methods            - Demonstrate GET method");
  println!("  PUT  /api/methods            - Demonstrate PUT method");
  println!("  DELETE /api/methods         - Demonstrate DELETE method");
  println!("  GET  /api/error              - Example error response");
  println!("  GET  /api/error/validation   - Validation error (400) (Task 16.8)");
  println!("  GET  /api/error/not-found    - Not found error (404) (Task 16.8)");
  println!("  GET  /api/error/custom       - Custom error (409) (Task 16.8)");
  println!("  POST /api/content-types     - Handle different content types");
  println!("  POST /api/stream-upload     - Stream large request bodies (Task 16.5)");
  println!("  GET  /api/stream-download   - Stream large response bodies (Task 16.6)");
  println!();

  axum::serve(listener, app).await?;

  Ok(())
}
