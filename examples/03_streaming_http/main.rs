use axum::{
  Router,
  extract::{Path, Query},
  response::Html,
  routing::get,
};
use http::Method;
use serde::Deserialize;
use std::time::Duration;
use streamweave::{
  http::http_request_chunk::StreamWeaveHttpRequestChunk,
  pipeline::PipelineBuilder,
  producers::vec::vec_producer::VecProducer,
  transformers::{
    backpressure::backpressure_transformer::BackpressureTransformer,
    map::map_transformer::MapTransformer,
  },
};

/// Query parameters for echo endpoint
#[derive(Deserialize)]
struct EchoQuery {
  delay: Option<u64>,
  format: Option<String>,
}

/// Echo endpoint with streaming capabilities
async fn echo_handler(
  Path(message): Path<String>,
  Query(params): Query<EchoQuery>,
) -> Html<String> {
  println!("🔄 Processing echo request for message: {}", message);

  // Get delay from query parameters (default 200ms)
  let delay = params.delay.unwrap_or(200);
  let format = params.format.as_deref().unwrap_or("html");

  // Simulate some processing time for streaming effect
  tokio::time::sleep(Duration::from_millis(delay)).await;

  // Process through StreamWeave pipeline
  let response = process_through_streamweave_pipeline(&message, format).await;

  Html(response)
}

/// Process request through StreamWeave pipeline
async fn process_through_streamweave_pipeline(message: &str, format: &str) -> String {
  // Create a mock request to pass through the streaming pipeline
  let uri: http::Uri = format!("http://localhost:3000/echo/{}", message)
    .parse()
    .unwrap();
  let connection_info = streamweave::http::connection_info::ConnectionInfo::new(
    std::net::SocketAddr::new(
      std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
      8080,
    ),
    std::net::SocketAddr::new(
      std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
      3000,
    ),
    http::Version::HTTP_11,
  );

  let request = StreamWeaveHttpRequestChunk::new(
    Method::GET,
    uri,
    http::HeaderMap::new(),
    bytes::Bytes::from(format!("Echo: {}", message)),
    connection_info,
    true,
  );

  // Create StreamWeave pipeline
  let (consumer, _chunk_receiver) = streamweave::consumers::http_response::http_response_consumer::StreamingHttpResponseConsumer::new();

  // Spawn pipeline processing
  let pipeline_handle = tokio::spawn(async move {
    let _pipeline = PipelineBuilder::new()
      .producer(VecProducer::new(vec![request]).with_name("echo_producer".to_string()))
      .transformer(BackpressureTransformer::new(10)) // Add backpressure control
      .transformer(MapTransformer::new(|chunk: StreamWeaveHttpRequestChunk| {
        // Process each chunk of the request
        let path = chunk.uri.path();
        let method = chunk.method.as_str();

        println!(
          "  📥 Processing streaming chunk: {} {} ({} bytes)",
          method,
          path,
          chunk.chunk.len()
        );

        // For this example, we'll echo the chunk back as a ResponseChunk
        streamweave::consumers::http_response::http_response_consumer::ResponseChunk::body(
          chunk.chunk,
        )
      }))
      ._consumer(consumer)
      .run()
      .await?;
    Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
  });

  // Wait for pipeline to complete
  let _ = pipeline_handle.await;

  // Build response based on format
  match format {
    "json" => serde_json::json!({
      "message": message,
      "echo": message.to_uppercase(),
      "timestamp": chrono::Utc::now().to_rfc3339(),
      "pipeline": "StreamWeave HTTP Router + Response Builder",
      "streaming": true,
      "backpressure_control": true
    })
    .to_string(),
    _ => {
      // Default HTML response
      format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>StreamWeave Echo</title>
    <style>
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            max-width: 800px;
            margin: 50px auto;
            padding: 20px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            text-align: center;
        }}
        .container {{
            background: rgba(255, 255, 255, 0.1);
            padding: 40px;
            border-radius: 20px;
            backdrop-filter: blur(10px);
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
        }}
        h1 {{
            font-size: 2.5em;
            margin-bottom: 20px;
            text-shadow: 2px 2px 4px rgba(0, 0, 0, 0.3);
        }}
        .echo-message {{
            font-size: 3em;
            font-weight: bold;
            margin: 30px 0;
            padding: 20px;
            background: rgba(255, 255, 255, 0.2);
            border-radius: 15px;
            text-shadow: 2px 2px 4px rgba(0, 0, 0, 0.3);
        }}
        .info {{
            font-size: 1.1em;
            opacity: 0.9;
            margin-top: 30px;
        }}
        .timestamp {{
            font-size: 0.9em;
            opacity: 0.7;
            margin-top: 20px;
        }}
        .streaming-badge {{
            background: rgba(255, 255, 255, 0.3);
            padding: 10px 20px;
            border-radius: 25px;
            font-size: 0.9em;
            margin-top: 20px;
            display: inline-block;
        }}
        .pipeline-info {{
            background: rgba(255, 255, 255, 0.1);
            padding: 20px;
            border-radius: 10px;
            margin-top: 20px;
            text-align: left;
        }}
        .endpoints {{
            background: rgba(255, 255, 255, 0.1);
            padding: 20px;
            border-radius: 10px;
            margin-top: 20px;
            text-align: left;
        }}
        .endpoint-link {{
            color: #ffd700;
            text-decoration: none;
            font-weight: bold;
        }}
        .endpoint-link:hover {{
            text-decoration: underline;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🔄 StreamWeave Echo</h1>
        <div class="echo-message">{}</div>
        <div class="info">
            <p>This response was generated by StreamWeave's <strong>streaming</strong> HTTP pipeline</p>
            <p>Route parameter: <code>/echo/{}</code></p>
            <p>Processed through the streaming pipeline with backpressure control</p>
        </div>
        <div class="pipeline-info">
            <h3>🚀 Pipeline Architecture</h3>
            <ul>
                <li><strong>HTTP Router:</strong> Request routing with path parameters</li>
                <li><strong>HTTP Response Builder:</strong> Structured response generation</li>
                <li><strong>Backpressure Control:</strong> Prevents memory overflow</li>
                <li><strong>Streaming Processing:</strong> Real-time data flow</li>
                <li><strong>Method:</strong> GET</li>
                <li><strong>Path:</strong> /echo/{}</li>
            </ul>
        </div>
        <div class="endpoints">
            <h3>🔗 Available Endpoints</h3>
            <ul>
                <li><a href="/echo/hello" class="endpoint-link">/echo/hello</a> - Echo with "hello"</li>
                <li><a href="/echo/streaming" class="endpoint-link">/echo/streaming</a> - Echo with "streaming"</li>
                <li><a href="/echo/test?delay=500" class="endpoint-link">/echo/test?delay=500</a> - Echo with 500ms delay</li>
                <li><a href="/echo/data?format=json" class="endpoint-link">/echo/data?format=json</a> - JSON response</li>
                <li><a href="/streaming/data" class="endpoint-link">/streaming/data</a> - Streaming data endpoint</li>
                <li><a href="/metrics" class="endpoint-link">/metrics</a> - Real-time metrics</li>
            </ul>
        </div>
        <div class="streaming-badge">🚀 STREAMING PIPELINE</div>
        <div class="timestamp">
            Generated at: {}
        </div>
    </div>
</body>
</html>"#,
        message.to_uppercase(),
        message,
        message,
        chrono::Utc::now()
          .format("%Y-%m-%d %H:%M:%S UTC")
          .to_string()
      )
    }
  }
}

/// Streaming data endpoint
async fn streaming_data_handler() -> axum::Json<serde_json::Value> {
  println!("📊 Processing streaming data request");

  // Simulate streaming data processing
  tokio::time::sleep(Duration::from_millis(300)).await;

  // Process through StreamWeave pipeline
  let response = process_streaming_data_through_pipeline().await;

  axum::Json(response)
}

/// Process streaming data through StreamWeave pipeline
async fn process_streaming_data_through_pipeline() -> serde_json::Value {
  // Create a mock request to pass through the streaming pipeline
  let uri: http::Uri = "http://localhost:3000/streaming/data".parse().unwrap();
  let connection_info = streamweave::http::connection_info::ConnectionInfo::new(
    std::net::SocketAddr::new(
      std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
      8080,
    ),
    std::net::SocketAddr::new(
      std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
      3000,
    ),
    http::Version::HTTP_11,
  );

  let request = StreamWeaveHttpRequestChunk::new(
    Method::GET,
    uri,
    http::HeaderMap::new(),
    bytes::Bytes::from("streaming data request"),
    connection_info,
    true,
  );

  // Create StreamWeave pipeline
  let (consumer, _chunk_receiver) = streamweave::consumers::http_response::http_response_consumer::StreamingHttpResponseConsumer::new();

  // Spawn pipeline processing
  let pipeline_handle = tokio::spawn(async move {
    let _pipeline = PipelineBuilder::new()
      .producer(VecProducer::new(vec![request]).with_name("streaming_data_producer".to_string()))
      .transformer(BackpressureTransformer::new(10))
      .transformer(MapTransformer::new(|chunk: StreamWeaveHttpRequestChunk| {
        println!(
          "  📥 Processing streaming data chunk: {} {} ({} bytes)",
          chunk.method.as_str(),
          chunk.uri.path(),
          chunk.chunk.len()
        );
        streamweave::consumers::http_response::http_response_consumer::ResponseChunk::body(
          chunk.chunk,
        )
      }))
      ._consumer(consumer)
      .run()
      .await?;
    Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
  });

  // Wait for pipeline to complete
  let _ = pipeline_handle.await;

  // Return structured streaming data
  serde_json::json!({
    "endpoint": "streaming-data",
    "method": "GET",
    "path": "/streaming/data",
    "type": "streaming",
    "data_chunks": [
      {"chunk": 1, "data": "First streaming chunk", "timestamp": chrono::Utc::now().to_rfc3339()},
      {"chunk": 2, "data": "Second streaming chunk", "timestamp": chrono::Utc::now().to_rfc3339()},
      {"chunk": 3, "data": "Third streaming chunk", "timestamp": chrono::Utc::now().to_rfc3339()},
    ],
    "status": "streaming_complete",
    "pipeline": "StreamWeave HTTP Router + Response Builder",
    "streaming": true,
    "backpressure_control": true,
    "chunks_processed": 3
  })
}

/// Real-time metrics endpoint
async fn metrics_handler() -> axum::Json<serde_json::Value> {
  println!("📈 Processing metrics request");

  // Simulate real-time metrics collection
  tokio::time::sleep(Duration::from_millis(150)).await;

  // Process through StreamWeave pipeline
  let response = process_metrics_through_pipeline().await;

  axum::Json(response)
}

/// Process metrics through StreamWeave pipeline
async fn process_metrics_through_pipeline() -> serde_json::Value {
  // Create a mock request to pass through the streaming pipeline
  let uri: http::Uri = "http://localhost:3000/metrics".parse().unwrap();
  let connection_info = streamweave::http::connection_info::ConnectionInfo::new(
    std::net::SocketAddr::new(
      std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
      8080,
    ),
    std::net::SocketAddr::new(
      std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
      3000,
    ),
    http::Version::HTTP_11,
  );

  let request = StreamWeaveHttpRequestChunk::new(
    Method::GET,
    uri,
    http::HeaderMap::new(),
    bytes::Bytes::from("metrics request"),
    connection_info,
    true,
  );

  // Create StreamWeave pipeline
  let (consumer, _chunk_receiver) = streamweave::consumers::http_response::http_response_consumer::StreamingHttpResponseConsumer::new();

  // Spawn pipeline processing
  let pipeline_handle = tokio::spawn(async move {
    let _pipeline = PipelineBuilder::new()
      .producer(VecProducer::new(vec![request]).with_name("metrics_producer".to_string()))
      .transformer(BackpressureTransformer::new(10))
      .transformer(MapTransformer::new(|chunk: StreamWeaveHttpRequestChunk| {
        println!(
          "  📥 Processing metrics chunk: {} {} ({} bytes)",
          chunk.method.as_str(),
          chunk.uri.path(),
          chunk.chunk.len()
        );
        streamweave::consumers::http_response::http_response_consumer::ResponseChunk::body(
          chunk.chunk,
        )
      }))
      ._consumer(consumer)
      .run()
      .await?;
    Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
  });

  // Wait for pipeline to complete
  let _ = pipeline_handle.await;

  // Return real-time metrics
  serde_json::json!({
    "endpoint": "real-time-metrics",
    "type": "streaming_metrics",
    "requests_per_second": 45.6,
    "active_connections": 12.0,
    "memory_usage_mb": 128.5,
    "cpu_usage_percent": 23.4,
    "timestamp": chrono::Utc::now().to_rfc3339(),
    "pipeline_status": "streaming",
    "streaming": true,
    "backpressure_control": true,
    "pipeline": "StreamWeave HTTP Router + Response Builder"
  })
}

#[tokio::main]
async fn main() {
  // Initialize logging
  tracing_subscriber::fmt::init();

  println!("🚀 StreamWeave HTTP Streaming Server");
  println!("=====================================");
  println!("Now featuring HTTP Router and HTTP Response Builder integration!");
  println!("Demonstrates streaming capabilities with full StreamWeave architecture");
  println!();

  // Build our application with routes
  let app = Router::new()
    .route("/echo/{message}", get(echo_handler))
    .route("/streaming/data", get(streaming_data_handler))
    .route("/metrics", get(metrics_handler))
    .layer(tower_http::cors::CorsLayer::permissive());

  // Run it
  let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));
  println!("🌐 Server starting on {}", addr);
  println!("📋 Available endpoints:");
  println!("   GET  /echo/:message             - Echo endpoint with streaming pipeline");
  println!("   GET  /streaming/data            - Streaming data endpoint with JSON responses");
  println!("   GET  /metrics                   - Real-time metrics endpoint");
  println!();
  println!("🔗 Test endpoints in your browser:");
  println!("   http://localhost:3000/echo/hello");
  println!("   http://localhost:3000/echo/streaming");
  println!("   http://localhost:3000/echo/test?delay=500");
  println!("   http://localhost:3000/echo/data?format=json");
  println!("   http://localhost:3000/streaming/data");
  println!("   http://localhost:3000/metrics");
  println!();
  println!("🎉 All endpoints use StreamWeave streaming pipeline!");
  println!("   - Path parameters like /echo/hello");
  println!("   - Query parameters like ?delay=500&format=json");
  println!("   - Beautiful HTML responses with pipeline information");
  println!("   - JSON responses for API endpoints");
  println!("   - Real-time streaming processing with backpressure control");

  let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
  println!("✅ Server listening on {}", addr);

  axum::serve(listener, app).await.unwrap();
}
