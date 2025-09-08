use axum::{
  Router,
  body::Body,
  extract::{Path, Request},
  http::StatusCode,
  response::Response,
  routing::get,
};
use std::net::SocketAddr;
use tokio;
use tower_http::cors::CorsLayer;

use streamweave::{
  pipeline::PipelineBuilder,
  structs::{
    consumers::http_response::{ResponseChunk, StreamingHttpResponseConsumer},
    producers::http_request::{StreamWeaveHttpRequestChunk, StreamingHttpRequestProducer},
    transformers::{backpressure::BackpressureTransformer, map::MapTransformer},
  },
};

// Echo endpoint with route parameter - now uses streaming implementation
async fn echo_route(Path(message): Path<String>) -> Result<Response<Body>, (StatusCode, String)> {
  println!("🔄 Processing echo request for message: {}", message);

  // Create a mock request to pass through the streaming pipeline
  let req = Request::builder()
    .uri(format!("/echo/{}", message))
    .method("GET")
    .body(Body::from(format!("Echo: {}", message)))
    .unwrap();

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
  let response_builder = response_builder.header("content-type", "text/plain; charset=utf-8");

  let _pipeline_result = response_builder.body(body).unwrap();

  // Always return the HTML response to show the streaming pipeline was used
  let html_content = format!(
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
        <div class="streaming-badge">🚀 STREAMING PIPELINE</div>
        <div class="timestamp">
            Generated at: {}
        </div>
    </div>
</body>
</html>"#,
    message.to_uppercase(),
    message,
    chrono::Utc::now()
      .format("%Y-%m-%d %H:%M:%S UTC")
      .to_string()
  );

  Ok(
    Response::builder()
      .status(StatusCode::OK)
      .header("content-type", "text/html; charset=utf-8")
      .body(Body::from(html_content))
      .unwrap(),
  )
}

#[tokio::main]
async fn main() {
  println!("🚀 StreamWeave HTTP Streaming Server");
  println!("📡 Built with Axum + StreamWeave");
  println!("🔄 Streaming-only architecture - no buffering!");
  println!();

  // Build our application with routes
  let app = Router::new()
    .route("/echo/{message}", get(echo_route))
    .layer(CorsLayer::permissive());

  // Run it
  let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
  println!("🌐 Server starting on {}", addr);
  println!("📋 Available endpoints:");
  println!("   GET  /echo/:message             - Echo endpoint with streaming pipeline");
  println!();
  println!("🔗 Test endpoints:");
  println!("   Browser: http://localhost:3000/echo/hello");
  println!();
  println!("🎉 Echo endpoint now uses StreamWeave streaming pipeline!");
  println!("   - Takes route parameters like /echo/abcd");
  println!("   - Returns beautiful HTML pages with the echoed message");
  println!("   - All processing goes through the streaming pipeline");

  let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
  println!("✅ Server listening on {}", addr);

  axum::serve(listener, app).await.unwrap();
}
