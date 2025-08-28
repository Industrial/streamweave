# StreamWeave HTTP Streaming Server

A real HTTP server built with **Axum** and **StreamWeave** that demonstrates streaming HTTP request processing through StreamWeave pipelines. This example focuses on **streaming-only architecture** with no buffering.

## 🚀 What This Example Shows

This example demonstrates how to build a **real HTTP server** where every request flows through a StreamWeave pipeline:

1. **HTTP Request** → **StreamWeave Producer** → **Pipeline** → **StreamWeave Consumer** → **HTTP Response**
2. **True streaming**: Each HTTP request is processed as a stream through the pipeline
3. **Real web server**: Built with Axum, listening on port 3000
4. **StreamWeave integration**: Uses our custom `StreamingHttpRequestProducer` and `StreamingHttpResponseConsumer`
5. **Browser-friendly**: All endpoints use GET requests for easy testing

## 🏗 Architecture

```
HTTP Client → Axum Router → StreamWeave Pipeline → HTTP Response
                    ↓
            StreamingHttpRequestProducer
                    ↓
            BackpressureTransformer (flow control)
                    ↓
            MapTransformer (business logic)
                    ↓
            StreamingHttpResponseConsumer
                    ↓
            HTTP Response
```

## 📋 Available Endpoints

- **GET** `/health` - Health check page (direct, no StreamWeave)
- **GET** `/echo/:message` - Echo endpoint with route parameter (direct, no StreamWeave)
- **GET** `/api/status` - API status page (direct, no StreamWeave)
- **GET** `/streaming/*` - Any path processed through StreamWeave streaming pipeline

## 🛠 How It Works

### 1. Echo Endpoint with Route Parameters

The echo endpoint demonstrates route parameter handling:

```rust
async fn echo_route(Path(message): Path<String>) -> Html<String> {
    let html_content = format!(
        r#"<!DOCTYPE html>
        <html>
            <body>
                <h1>🚀 StreamWeave Echo</h1>
                <div class="echo-message">{}</div>
            </body>
        </html>"#,
        message.to_uppercase()
    );
    Html(html_content)
}
```

**Usage**: Visit `/echo/hello` in your browser to see "HELLO" displayed in a beautiful HTML page.

### 2. Streaming Pipeline Processing

For the `/streaming/*` endpoints, requests flow through the StreamWeave pipeline:

```rust
async fn process_request_through_streamweave_streaming(req: Request) -> Result<Response<Body>, (StatusCode, String)> {
    // Create streaming StreamWeave pipeline
    let (consumer, mut chunk_receiver) = StreamingHttpResponseConsumer::new();
    
    let pipeline_handle = tokio::spawn(async move {
        let _pipeline = PipelineBuilder::new()
            .producer(StreamingHttpRequestProducer::from_axum_request(req).await)
            .transformer(BackpressureTransformer::new(10)) // Flow control
            .transformer(MapTransformer::new(|chunk| {
                // Process each chunk of the request
                ResponseChunk::body(chunk.chunk)
            }))
            ._consumer(consumer)
            .run()
            .await?;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });
    
    // Process streaming response chunks
    // ... chunk processing logic
}
```

### 3. StreamWeave Components

- **`StreamingHttpRequestProducer`**: Converts Axum `Request` into streaming chunks
- **`BackpressureTransformer`**: Controls flow with configurable buffer sizes
- **`MapTransformer`**: Applies business logic to each chunk
- **`StreamingHttpResponseConsumer`**: Collects streaming response chunks

## 🚀 Running the Example

```bash
# Build and run
cargo run --example 03_streaming_http

# Or build first, then run
cargo build --example 03_streaming_http
./target/debug/examples/03_streaming_http
```

The server will start on `http://localhost:3000`

## 🧪 Testing

### Browser Testing (Recommended)

All endpoints are now GET requests, making them perfect for browser testing:

- **Health Check**: http://localhost:3000/health
- **Echo Test**: http://localhost:3000/echo/hello
- **API Status**: http://localhost:3000/api/status
- **Streaming Test**: http://localhost:3000/streaming/test

### Command Line Testing

```bash
# Health check
curl http://localhost:3000/health

# Echo endpoint with route parameter
curl http://localhost:3000/echo/world

# API status
curl http://localhost:3000/api/status

# Streaming endpoint
curl http://localhost:3000/streaming/example
```

## 💡 Key Benefits

1. **True Streaming**: HTTP requests flow as streams through StreamWeave with no buffering
2. **Browser-Friendly**: All endpoints use GET requests for easy testing
3. **Route Parameters**: Echo endpoint demonstrates parameter handling with `/echo/:message`
4. **Beautiful HTML**: Returns styled HTML pages instead of plain JSON
5. **Flow Control**: Backpressure transformer prevents memory issues
6. **Real HTTP Server**: Actual web server with Axum, not a simulation

## 🔧 Extending the Example

### Add More Route Parameters

```rust
// Multiple parameters
async fn user_profile(Path((username, section)): Path<(String, String)>) -> Html<String> {
    // Handle /user/:username/:section
}

// Optional parameters
async fn search(Path(query): Path<Option<String>>) -> Html<String> {
    let query = query.unwrap_or_else(|| "default".to_string());
    // Handle /search or /search/query
}
```

### Add More Transformers

```rust
let pipeline = PipelineBuilder::new()
    .producer(StreamingHttpRequestProducer::from_axum_request(req).await)
    .transformer(BackpressureTransformer::new(100))        // Larger buffer
    .transformer(RateLimitTransformer::new(100, 60))      // Rate limiting
    .transformer(CircuitBreakerTransformer::new(3, Duration::from_secs(5))) // Circuit breaker
    .transformer(MapTransformer::new(business_logic))     // Business logic
    ._consumer(consumer)
    .run()
    .await?;
```

### Add Middleware

```rust
let app = Router::new()
    .route("/echo/:message", get(echo_route))
    .route("/streaming/*path", get(process_request_through_streamweave_streaming))
    .layer(CorsLayer::permissive())
    .layer(tower_http::trace::TraceLayer::new_for_http())
    .layer(tower_http::compression::CompressionLayer::new());
```

## 🎯 Real-World Applications

This pattern is perfect for:

- **API Gateways**: Route and transform incoming requests with streaming
- **Content Delivery**: Stream large files or media content
- **Real-time Processing**: Process data as it arrives
- **Microservices**: Handle requests through business logic pipelines
- **Middleware Chains**: Authentication, validation, logging with streaming

## 🚧 Technical Notes

- **Axum Integration**: Uses Axum 0.8+ with modern async patterns
- **StreamWeave Compatibility**: All types implement required traits (`Clone`, `Send`, `Sync`)
- **Streaming Architecture**: No buffering - true streaming from request to response
- **Backpressure Control**: Configurable buffer sizes prevent memory issues
- **HTML Responses**: Beautiful, styled HTML pages for better user experience
- **CORS Support**: Includes permissive CORS for testing

## 🔮 Next Steps

1. **Add Authentication**: Implement JWT or session-based auth in transformers
2. **Add Logging**: Create logging transformers for request/response tracking
3. **Add Metrics**: Implement metrics collection transformers
4. **Add Caching**: Create cache transformers for response caching
5. **Add Rate Limiting**: Implement rate limiting transformers
6. **Add Circuit Breakers**: Use existing circuit breaker transformers
7. **Add WebSocket Support**: Extend for real-time bidirectional communication

## 🎨 UI Features

The example includes beautiful HTML responses with:

- **Responsive Design**: Works on desktop and mobile devices
- **Modern Styling**: Gradient backgrounds and glass-morphism effects
- **Interactive Elements**: Hover effects and visual feedback
- **Professional Layout**: Clean, centered design with proper typography
- **Real-time Data**: Timestamps and dynamic content

This example demonstrates the power of combining **Axum's excellent HTTP handling** with **StreamWeave's streaming pipeline capabilities** to create a truly streaming HTTP server that's both powerful and user-friendly! 🎉
