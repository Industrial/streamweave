# StreamWeave HTTP Streaming Server

A real HTTP server built with **Axum** and **StreamWeave** that demonstrates streaming HTTP request processing through StreamWeave pipelines.

## 🚀 What This Example Shows

This example demonstrates how to build a **real HTTP server** where every request flows through a StreamWeave pipeline:

1. **HTTP Request** → **StreamWeave Producer** → **Pipeline** → **StreamWeave Consumer** → **HTTP Response**
2. **True streaming**: Each HTTP request is processed as a stream through the pipeline
3. **Real web server**: Built with Axum, listening on port 3000
4. **StreamWeave integration**: Uses our custom `HttpRequestProducer` and `HttpResponseConsumer`

## 🏗 Architecture

```
HTTP Client → Axum Router → StreamWeave Pipeline → HTTP Response
                    ↓
            HttpRequestProducer
                    ↓
            MapTransformer (business logic)
                    ↓
            HttpResponseConsumer
                    ↓
            HTTP Response
```

## 📋 Available Endpoints

- **GET** `/health` - Direct health check (bypasses StreamWeave)
- **POST** `/echo` - Echo endpoint processed through StreamWeave pipeline
- **GET** `/api/status` - API status processed through StreamWeave pipeline  
- **ANY** `/streamweave/*` - Any path processed through StreamWeave pipeline

## 🛠 How It Works

### 1. Request Processing Flow

```rust
async fn process_request_through_streamweave(req: Request) -> Result<Response<Body>, (StatusCode, String)> {
    // Create StreamWeave pipeline
    let (consumer, response_receiver) = HttpResponseConsumer::new();
    
    let pipeline = PipelineBuilder::new()
        .producer(HttpRequestProducer::from_axum_request(req).await)
        .transformer(MapTransformer::new(|streamweave_req| {
            // Transform HTTP request into response
            // Business logic here!
        }))
        .consumer(consumer)
        .run()
        .await
        .unwrap();
    
    // Get response from pipeline
    let streamweave_response = response_receiver.await?;
    Ok(streamweave_response.into_axum_response())
}
```

### 2. StreamWeave Components

- **`HttpRequestProducer`**: Converts Axum `Request` into `StreamWeaveHttpRequest`
- **`MapTransformer`**: Applies business logic (routing, response generation)
- **`HttpResponseConsumer`**: Collects the response and sends it back

### 3. Type Safety

All types implement `Clone` for StreamWeave compatibility:
- `StreamWeaveHttpRequest`: Clone-able HTTP request representation
- `StreamWeaveHttpResponse`: Clone-able HTTP response representation

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

```bash
# Health check (direct, no StreamWeave)
curl http://localhost:3000/health

# Echo endpoint (via StreamWeave pipeline)
curl -X POST http://localhost:3000/echo -d "Hello World"

# API status (via StreamWeave pipeline)
curl http://localhost:3000/api/status

# Any path via StreamWeave
curl http://localhost:3000/streamweave/custom/path
```

## 💡 Key Benefits

1. **True Streaming**: HTTP requests flow as streams through StreamWeave
2. **Type Safety**: All components implement `Clone` for StreamWeave compatibility
3. **Real HTTP Server**: Not a simulation - actual web server with Axum
4. **Extensible**: Easy to add more transformers (auth, logging, rate limiting)
5. **Performance**: Leverages Axum's high-performance async runtime

## 🔧 Extending the Example

### Add More Transformers

```rust
let pipeline = PipelineBuilder::new()
    .producer(HttpRequestProducer::from_axum_request(req).await)
    .transformer(LoggingTransformer::new())           // Log requests
    .transformer(AuthTransformer::new())              // Authenticate
    .transformer(RateLimitTransformer::new(100, 60)) // Rate limit
    .transformer(MapTransformer::new(business_logic)) // Business logic
    .transformer(LoggingTransformer::new())           // Log responses
    .consumer(consumer)
    .run()
    .await?;
```

### Add Middleware

```rust
let app = Router::new()
    .route("/api/*", post(process_request_through_streamweave))
    .layer(CorsLayer::permissive())
    .layer(tower_http::trace::TraceLayer::new_for_http())
    .layer(tower_http::compression::CompressionLayer::new());
```

## 🎯 Real-World Applications

This pattern is perfect for:

- **API Gateways**: Route and transform incoming requests
- **Microservices**: Process requests through business logic pipelines
- **Middleware Chains**: Authentication, validation, logging, etc.
- **Request Processing**: Complex business logic with streaming
- **Response Transformation**: Format conversion, filtering, enrichment

## 🚧 Technical Notes

- **Axum Integration**: Uses Axum 0.8+ with modern async patterns
- **StreamWeave Compatibility**: All types implement required traits (`Clone`, `Send`, `Sync`)
- **HTTP Body Handling**: Converts between Axum and StreamWeave body types
- **Error Handling**: Proper error propagation through the pipeline
- **CORS Support**: Includes permissive CORS for testing

## 🔮 Next Steps

1. **Add Authentication**: Implement JWT or session-based auth in transformers
2. **Add Logging**: Create logging transformers for request/response tracking
3. **Add Metrics**: Implement metrics collection transformers
4. **Add Caching**: Create cache transformers for response caching
5. **Add Rate Limiting**: Implement rate limiting transformers
6. **Add Circuit Breakers**: Use existing circuit breaker transformers

This example demonstrates the power of combining **Axum's excellent HTTP handling** with **StreamWeave's streaming pipeline capabilities** to create a truly streaming HTTP server! 🎉
