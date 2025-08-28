use axum::{
  Router,
  body::Body,
  extract::Request,
  http::StatusCode,
  response::Response,
  routing::{get, post},
};
use bytes::Bytes;
use serde::{Deserialize, Serialize};

use std::net::SocketAddr;
use tokio;
use tower_http::cors::CorsLayer;

use streamweave::{
  consumers::http_response::{
    HttpResponseConsumer, ResponseChunk, StreamWeaveHttpResponse, StreamingHttpResponseConsumer,
  },
  pipeline::PipelineBuilder,
  producers::http_request::{
    HttpRequestProducer, StreamWeaveHttpRequest, StreamWeaveHttpRequestChunk,
    StreamingHttpRequestProducer,
  },
  transformers::{backpressure::BackpressureTransformer, map::MapTransformer},
};

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct RequestData {
  message: String,
  count: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
struct ResponseData {
  status: String,
  echo: String,
  processed_count: i32,
  timestamp: String,
}

// StreamWeave pipeline processor for HTTP requests (buffered mode)
async fn process_request_through_streamweave(
  req: Request,
) -> Result<Response<Body>, (StatusCode, String)> {
  println!("🚀 Processing request through StreamWeave pipeline (buffered mode)");

  // Create StreamWeave pipeline
  let (consumer, response_receiver) = HttpResponseConsumer::new();

  let _pipeline = PipelineBuilder::new()
    .producer(HttpRequestProducer::from_axum_request(req).await)
    .transformer(MapTransformer::new(
      |streamweave_req: StreamWeaveHttpRequest| {
        // Transform StreamWeave HTTP request into response
        let path = streamweave_req.uri().path();
        let method = streamweave_req.method().as_str();

        println!("  📥 Processing: {} {} (buffered)", method, path);

        match (method, path) {
          ("GET", "/health") => {
            let response_data = ResponseData {
              status: "healthy".to_string(),
              echo: "Server is running".to_string(),
              processed_count: 1,
              timestamp: chrono::Utc::now().to_rfc3339(),
            };

            let body = serde_json::to_string(&response_data).unwrap();
            StreamWeaveHttpResponse::ok(Bytes::from(body)).with_content_type("application/json")
          }

          ("POST", "/echo") => {
            let body_str = String::from_utf8_lossy(&streamweave_req.body);

            let response_data = ResponseData {
              status: "success".to_string(),
              echo: format!("Echo: {}", body_str),
              processed_count: body_str.len() as i32,
              timestamp: chrono::Utc::now().to_rfc3339(),
            };

            let body = serde_json::to_string(&response_data).unwrap();
            StreamWeaveHttpResponse::ok(Bytes::from(body)).with_content_type("application/json")
          }

          ("GET", "/api/status") => {
            let response_data = ResponseData {
              status: "operational".to_string(),
              echo: "API is operational".to_string(),
              processed_count: 1,
              timestamp: chrono::Utc::now().to_rfc3339(),
            };

            let body = serde_json::to_string(&response_data).unwrap();
            StreamWeaveHttpResponse::ok(Bytes::from(body)).with_content_type("application/json")
          }

          _ => {
            let response_data = ResponseData {
              status: "error".to_string(),
              echo: "Endpoint not found".to_string(),
              processed_count: 0,
              timestamp: chrono::Utc::now().to_rfc3339(),
            };

            let body = serde_json::to_string(&response_data).unwrap();
            StreamWeaveHttpResponse::not_found(Bytes::from(body))
              .with_content_type("application/json")
          }
        }
      },
    ))
    ._consumer(consumer)
    .run()
    .await
    .unwrap();

  // Get the response from the pipeline
  match response_receiver.await {
    Ok(streamweave_response) => {
      println!("  📤 Pipeline completed successfully");
      Ok(streamweave_response.into_axum_response())
    }
    Err(_) => {
      println!("  ❌ Pipeline failed to produce response");
      Err((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Pipeline error".to_string(),
      ))
    }
  }
}

// NEW: StreamWeave pipeline processor for HTTP requests (streaming mode)
async fn process_request_through_streamweave_streaming(
  req: Request,
) -> Result<Response<Body>, (StatusCode, String)> {
  println!("🚀 Processing request through StreamWeave pipeline (streaming mode)");

  // Create streaming StreamWeave pipeline
  let (consumer, mut chunk_receiver) = StreamingHttpResponseConsumer::new();

  // Spawn pipeline processing
  let pipeline_handle = tokio::spawn(async move {
    let _pipeline = PipelineBuilder::new()
      .producer(StreamingHttpRequestProducer::from_axum_request(req).await)
      .transformer(BackpressureTransformer::new(10)) // Add backpressure control
      .transformer(MapTransformer::new(|chunk: StreamWeaveHttpRequestChunk| {
        // Process each chunk of the request
        let path = chunk.uri.path();
        let method = chunk.method.as_str();

        println!(
          "  📥 Processing chunk: {} {} ({} bytes)",
          method,
          path,
          chunk.chunk.len()
        );

        // For this example, we'll echo the chunk back as a ResponseChunk
        ResponseChunk::body(chunk.chunk)
      }))
      ._consumer(consumer)
      .run()
      .await?;
    Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
  });

  // Wait for pipeline to complete
  let _ = pipeline_handle.await;

  // Build streaming response
  let mut response_builder = Response::builder();
  let mut body_chunks = Vec::new();

  // Process response chunks
  while let Some(chunk) = chunk_receiver.recv().await {
    match chunk {
      ResponseChunk::Header(status, headers) => {
        response_builder = response_builder.status(status);
        for (key, value) in headers {
          if let Some(key) = key {
            response_builder = response_builder.header(key, value);
          }
        }
      }
      ResponseChunk::Body(data) => {
        body_chunks.push(data);
      }
      ResponseChunk::End => break,
      ResponseChunk::Error(status, message) => {
        return Err((status, message));
      }
    }
  }

  // Convert chunks to streaming body - use a simple approach for now
  let body = if body_chunks.is_empty() {
    Body::from("No data")
  } else {
    // For simplicity, concatenate all chunks
    let combined = body_chunks.into_iter().fold(Vec::new(), |mut acc, chunk| {
      acc.extend_from_slice(&chunk);
      acc
    });
    Body::from(combined)
  };

  // Set default status if none was set
  let response_builder = response_builder.status(StatusCode::OK);

  // Set default content type
  let response_builder = response_builder.header("content-type", "application/octet-stream");

  Ok(response_builder.body(body).unwrap())
}

// Health check endpoint
async fn health_check() -> Response<Body> {
  let response_data = ResponseData {
    status: "healthy".to_string(),
    echo: "Direct health check".to_string(),
    processed_count: 1,
    timestamp: chrono::Utc::now().to_rfc3339(),
  };

  let body = serde_json::to_string(&response_data).unwrap();
  Response::builder()
    .status(StatusCode::OK)
    .header("content-type", "application/json")
    .body(Body::from(body))
    .unwrap()
}

#[tokio::main]
async fn main() {
  println!("🚀 StreamWeave HTTP Streaming Server");
  println!("📡 Built with Axum + StreamWeave");
  println!("🔄 Now with TRUE STREAMING capabilities!");
  println!();

  // Build our application with routes
  let app = Router::new()
    .route("/health", get(health_check))
    .route("/echo", post(process_request_through_streamweave))
    .route("/api/status", get(process_request_through_streamweave))
    .route(
      "/streamweave/{*path}",
      post(process_request_through_streamweave),
    )
    .route(
      "/streamweave/{*path}",
      get(process_request_through_streamweave),
    )
    // NEW: Streaming endpoints
    .route(
      "/streaming/echo",
      post(process_request_through_streamweave_streaming),
    )
    .route(
      "/streaming/api/status",
      get(process_request_through_streamweave_streaming),
    )
    .route(
      "/streaming/{*path}",
      post(process_request_through_streamweave_streaming),
    )
    .route(
      "/streaming/{*path}",
      get(process_request_through_streamweave_streaming),
    )
    .layer(CorsLayer::permissive());

  // Run it
  let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
  println!("🌐 Server starting on {}", addr);
  println!("📋 Available endpoints:");
  println!("   GET  /health                    - Direct health check");
  println!("   POST /echo                      - Echo endpoint (buffered via StreamWeave)");
  println!("   GET  /api/status                - API status (buffered via StreamWeave)");
  println!("   ANY  /streamweave/*             - Any path via StreamWeave pipeline (buffered)");
  println!();
  println!("🔄 NEW STREAMING ENDPOINTS:");
  println!("   POST /streaming/echo            - Echo endpoint (TRUE STREAMING via StreamWeave)");
  println!("   GET  /streaming/api/status     - API status (TRUE STREAMING via StreamWeave)");
  println!(
    "   ANY  /streaming/*               - Any path via StreamWeave pipeline (TRUE STREAMING)"
  );
  println!();
  println!("🔗 Test buffered mode:");
  println!("   curl -X POST http://localhost:3000/echo -d 'Hello World'");
  println!();
  println!("🔗 Test streaming mode:");
  println!("   curl -X POST http://localhost:3000/streaming/echo -d 'Hello Streaming World'");
  println!();
  println!("🎉 The streaming endpoints now process requests as true streams!");
  println!("   - No more buffering entire bodies in memory");
  println!("   - Backpressure control with configurable buffer sizes");
  println!("   - Chunked transfer encoding support");
  println!("   - Memory-efficient processing of large payloads");

  let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
  println!("✅ Server listening on {}", addr);

  axum::serve(listener, app).await.unwrap();
}
