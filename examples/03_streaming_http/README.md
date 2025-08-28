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

- **GET** `/echo/{message}` - Echo endpoint with route parameter (processed through StreamWeave streaming pipeline)

## 🛠 How It Works

### Echo Endpoint with Route Parameters

The echo endpoint demonstrates route parameter handling and streaming pipeline processing:

```rust
async fn echo_route(Path(message): Path<String>) -> Result<Response<Body>, (StatusCode, String)> {
    // Create a mock request to pass through the streaming pipeline
    let req = Request::builder()
        .uri(format!("/echo/{}", message))
        .method("GET")
        .body(Body::from(format!("Echo: {}", message)))
        .unwrap();

    // Process through the streaming pipeline
    let result = process_request_through_streamweave_streaming(req).await;
    
    // Fall back to HTML response if pipeline fails
    if let Err(_) = result {
        // Generate beautiful HTML response
    }
    
    result
}
```

**Usage**: Visit `/echo/hello` in your browser to see "HELLO" displayed in a beautiful HTML page, processed through the StreamWeave streaming pipeline.

### Streaming Pipeline Processing

All requests flow through the StreamWeave pipeline:

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

The server will start on `http://localhost:3000` with the following endpoint:

- **GET** `/echo/:message` - Echo endpoint processed through streaming pipeline

## 🎯 Key Features

### 1. **Single Route Architecture**
- Only one endpoint: `/echo/:message`
- All requests go through the StreamWeave streaming pipeline
- Simplified routing for focused demonstration

### 2. **Streaming Pipeline Integration**
- Every request flows through the complete StreamWeave pipeline
- Demonstrates real-world streaming architecture
- Includes backpressure control and error handling

### 3. **Beautiful HTML Responses**
- Modern, responsive design with gradients and animations
- Shows streaming pipeline status and processing information
- Easy to test in any web browser

### 4. **Robust Error Handling**
- Fallback to HTML response if pipeline processing fails
- Graceful degradation ensures the endpoint always responds
- Comprehensive error logging and debugging

## 🔧 Customization

### Modify the Pipeline

You can easily customize the StreamWeave pipeline by adding more transformers:

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
5. **Add More Routes**: Extend with additional endpoints that use the same streaming pattern
6. **Add Database Integration**: Connect to databases through StreamWeave producers/consumers

## 🎉 Summary

This example demonstrates a **production-ready streaming architecture** where:

- **Single endpoint** (`/echo/:message`) showcases the complete streaming pipeline
- **Every request** flows through StreamWeave's streaming infrastructure
- **Real streaming** with backpressure control and error handling
- **Beautiful UI** that clearly shows the streaming pipeline in action
- **Easy testing** with simple GET requests in any web browser

The architecture is designed to be easily extended with additional routes, all following the same streaming pattern through StreamWeave's powerful pipeline system.
