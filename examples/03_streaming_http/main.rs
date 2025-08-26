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
  consumers::http_response::{HttpResponseConsumer, StreamWeaveHttpResponse},
  pipeline::PipelineBuilder,
  producers::http_request::{HttpRequestProducer, StreamWeaveHttpRequest},
  transformers::map::MapTransformer,
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

// StreamWeave pipeline processor for HTTP requests
async fn process_request_through_streamweave(
  req: Request,
) -> Result<Response<Body>, (StatusCode, String)> {
  println!("🚀 Processing request through StreamWeave pipeline");

  // Create StreamWeave pipeline
  let (consumer, response_receiver) = HttpResponseConsumer::new();

  let _pipeline = PipelineBuilder::new()
    .producer(HttpRequestProducer::from_axum_request(req).await)
    .transformer(MapTransformer::new(
      |streamweave_req: StreamWeaveHttpRequest| {
        // Transform StreamWeave HTTP request into response
        let path = streamweave_req.uri.path();
        let method = streamweave_req.method.as_str();

        println!("  📥 Processing: {} {}", method, path);

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
  println!();

  // Build our application with a route
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
    .layer(CorsLayer::permissive());

  // Run it
  let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
  println!("🌐 Server starting on {}", addr);
  println!("📋 Available endpoints:");
  println!("   GET  /health                    - Direct health check");
  println!("   POST /echo                      - Echo endpoint (via StreamWeave)");
  println!("   GET  /api/status                - API status (via StreamWeave)");
  println!("   ANY  /streamweave/*             - Any path via StreamWeave pipeline");
  println!();
  println!("� The StreamWeave pipeline processes requests as streams!");
  println!("🔗 Try: curl -X POST http://localhost:3000/echo -d 'Hello World'");
  println!();

  let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
  println!("✅ Server listening on {}", addr);

  axum::serve(listener, app).await.unwrap();
}
