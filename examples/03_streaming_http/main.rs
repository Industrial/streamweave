use bytes::Bytes;
use http::StatusCode;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use streamweave::{
  consumers::http_server_response::{HttpServerResponse, HttpServerResponseConsumer},
  http::connection::ConnectionManager,
  http::http_request_chunk::StreamWeaveHttpRequestChunk,
  pipeline::PipelineBuilder,
  producers::http_server_producer::http_server_producer::HttpServerProducer,
  transformers::map::map_transformer::MapTransformer,
};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Initialize logging
  tracing_subscriber::fmt::init();

  println!("🚀 StreamWeave Complete HTTP Server Example");
  println!("==========================================");
  println!("Demonstrating complete HTTP request/response cycle with StreamWeave");
  println!();

  // Create connection manager
  let connection_manager = Arc::new(ConnectionManager::new(100));

  // Create HTTP server producer
  let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
  let http_producer = HttpServerProducer::bind(addr)
    .await?
    .with_max_connections(100)
    .with_connection_timeout(Duration::from_secs(30))
    .with_keep_alive_timeout(Duration::from_secs(60));

  // Create HTTP server response consumer
  let response_consumer = HttpServerResponseConsumer::new(connection_manager.clone());

  // Create a transformer that converts HTTP requests to HTTP responses
  let request_to_response_transformer = MapTransformer::new(
    |request: StreamWeaveHttpRequestChunk| -> HttpServerResponse {
      info!("📥 Processing request: {} {}", request.method, request.uri);

      let response_body = match request.uri.path() {
        "/health" => {
          info!("💚 Health check requested");
          let timestamp = chrono::Utc::now().to_rfc3339();
          let response = format!("{{\"status\":\"healthy\",\"timestamp\":\"{}\"}}", timestamp);
          Bytes::from(response)
        }
        "/echo" => {
          let echo_response = format!(
            "{{\"echo\":\"Hello from StreamWeave!\",\"method\":\"{}\",\"path\":\"{}\"}}",
            request.method,
            request.uri.path()
          );
          Bytes::from(echo_response)
        }
        _ => Bytes::from("{\"error\":\"Not Found\"}"),
      };

      let status = if request.uri.path() == "/health" || request.uri.path() == "/echo" {
        StatusCode::OK
      } else {
        StatusCode::NOT_FOUND
      };

      HttpServerResponse::new(
        request.request_id,
        status,
        http::HeaderMap::new(),
        response_body,
      )
      .with_header("content-type", "application/json")
    },
  );

  // The HTTP server producer will be used directly in the pipeline

  println!("🌐 HTTP Server listening on http://{}", addr);
  println!("📡 Available endpoints:");
  println!("   GET /health - Health check");
  println!("   GET /echo   - Echo service");
  println!("   GET /*      - 404 Not Found");
  println!();
  println!("🔗 Test with:");
  println!("   curl http://{}/health", addr);
  println!("   curl http://{}/echo", addr);
  println!();

  // Create and run the complete pipeline
  let pipeline = PipelineBuilder::new()
    .producer(http_producer)
    .transformer(request_to_response_transformer)
    ._consumer(response_consumer);

  // Run the pipeline
  info!("🚀 Starting StreamWeave pipeline...");
  let _ = pipeline.run().await;

  Ok(())
}
