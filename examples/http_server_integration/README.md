# HTTP Server Integration Example

This example demonstrates how to use StreamWeave's HTTP server capabilities to build REST microservices with Axum integration. It covers all the core HTTP server features including request/response handling, pipeline integration, and various HTTP methods.

## Prerequisites

Before running this example, you need:

1. **HTTP server feature enabled** - Build with `--features http-server`
2. **Tokio runtime** - Required for async HTTP server

## Quick Start

```bash
# Run the HTTP server example
cargo run --example http_server_integration --features http-server
```

The server will start on `http://127.0.0.1:3000` and you can test the endpoints using curl or any HTTP client.

## What This Example Demonstrates

### Task 16.1: HTTP Server Types

This example demonstrates the core HTTP server types:

- **HttpRequest**: Wraps Axum requests with metadata (method, path, headers, query params, body)
- **HttpResponse**: Constructs HTTP responses with status codes, headers, and body
- **HttpMethod**: Type-safe HTTP method enumeration (GET, POST, PUT, DELETE, etc.)
- **ContentType**: Content type handling (JSON, text, binary, etc.)

**Example Usage:**
```rust
// Create an HTTP request from Axum request
let request = HttpRequest::from_axum_request(axum_request).await;

// Access request metadata
let method = request.method;
let path = request.path;
let query_params = request.get_query_param("id");

// Create responses
let response = HttpResponse::json(StatusCode::OK, &data)?;
let text_response = HttpResponse::text(StatusCode::OK, "Hello");
let error_response = HttpResponse::error(StatusCode::BAD_REQUEST, "Error message");
```

### Task 16.2: HTTP Request Producer

This example demonstrates the `HttpRequestProducer` which converts incoming HTTP requests into stream items:

- Extracts request metadata (method, path, headers)
- Parses query parameters
- Extracts request body (JSON or raw bytes)
- Supports concurrent requests
- Handles errors gracefully

**Example Usage:**
```rust
let producer = HttpRequestProducer::from_axum_request(
    axum_request,
    HttpRequestProducerConfig::default()
        .with_extract_body(true)
        .with_parse_json(true),
).await;

let stream = producer.produce();
// Stream yields HttpRequest items
```

**Endpoints demonstrating this:**
- `GET /api/query?name=test&age=25` - Query parameter extraction
- `POST /api/body` - Request body extraction

### Task 16.3: HTTP Response Consumer

This example demonstrates the `HttpResponseConsumer` which converts stream items into HTTP responses:

- Sets HTTP status codes
- Sets response headers
- Handles different content types (JSON, text, binary)
- Supports single-item and multi-item responses
- Creates error responses with appropriate status codes

**Example Usage:**
```rust
let consumer = HttpResponseConsumer::new();
// ... consume stream ...
let axum_response = consumer.get_response().await;
```

**Endpoints demonstrating this:**
- `POST /api/process` - Pipeline processing with response consumer
- `GET /api/error` - Error response creation

### Task 16.4: Axum Route Handler Integration

This example demonstrates Axum route handlers that integrate HTTP requests/responses with StreamWeave pipelines:

- `create_simple_handler`: Simple request-to-response transformation
- `create_pipeline_handler`: Full pipeline integration with transformers
- Multiple HTTP methods (GET, POST, PUT, DELETE)
- Async request handling
- Error handling and conversion to HTTP errors

**Example Usage:**
```rust
// Simple handler
let app = Router::new()
    .route("/api/echo", get(create_simple_handler(|req: HttpRequest| {
        HttpResponse::text(StatusCode::OK, "Echo")
    })));

// Pipeline handler
let app = Router::new()
    .route("/api/process", post(create_pipeline_handler(|| {
        MapTransformer::new(|req: HttpRequest| {
            HttpResponse::json(StatusCode::OK, &process(req))
        })
    })));
```

## Testing the Endpoints

Once the server is running, you can test the endpoints:

### 1. Echo Endpoint (Task 16.1 & 16.4)

```bash
curl http://127.0.0.1:3000/api/echo
```

**Response:**
```json
{
  "method": "Get",
  "path": "/api/echo",
  "query_params": {},
  "headers_count": 8,
  "message": "Echo endpoint - demonstrates HttpRequest and HttpResponse types"
}
```

### 2. Content Type Detection (Task 16.1)

```bash
curl -X POST http://127.0.0.1:3000/api/content-type \
  -H "Content-Type: application/json" \
  -d '{"test": "data"}'
```

**Response:**
```json
{
  "content_type": "Json",
  "body_size": 16,
  "message": "Content type detection example"
}
```

### 3. Query Parameters (Task 16.2)

```bash
curl "http://127.0.0.1:3000/api/query?name=Alice&age=30"
```

**Response:**
```json
{
  "name": "Alice",
  "age": 30,
  "all_params": {
    "name": "Alice",
    "age": "30"
  },
  "message": "Query parameter extraction example"
}
```

### 4. Request Body Extraction (Task 16.2)

```bash
curl -X POST http://127.0.0.1:3000/api/body \
  -H "Content-Type: application/json" \
  -d '{"message": "Hello, StreamWeave!"}'
```

**Response:**
```json
{
  "body_received": true,
  "body_length": 29,
  "body_preview": "{\"message\": \"Hello, StreamWeave!\"}",
  "message": "Request body extraction example"
}
```

### 5. Pipeline Processing (Task 16.3 & 16.4)

```bash
curl -X POST http://127.0.0.1:3000/api/process \
  -H "Content-Type: application/json" \
  -d '{"data": "test"}'
```

**Response:**
```json
{
  "original_path": "/api/process",
  "method": "Post",
  "processed_at": "2024-01-15T10:30:00Z",
  "body_size": 16
}
```

### 6. Different HTTP Methods (Task 16.1 & 16.4)

```bash
# GET
curl http://127.0.0.1:3000/api/methods

# PUT
curl -X PUT http://127.0.0.1:3000/api/methods

# DELETE
curl -X DELETE http://127.0.0.1:3000/api/methods
```

### 7. Error Response (Task 16.3)

```bash
curl http://127.0.0.1:3000/api/error
```

**Response:**
```json
{
  "error": "This is an example error response",
  "status": 400
}
```

### 8. Content Types (Task 16.1)

```bash
# JSON
curl -X POST http://127.0.0.1:3000/api/content-types \
  -H "Content-Type: application/json" \
  -d '{"test": "data"}'

# Text
curl -X POST http://127.0.0.1:3000/api/content-types \
  -H "Content-Type: text/plain" \
  -d "Hello, world!"
```

## Code Examples

### Creating HTTP Requests and Responses

```rust
use streamweave::http_server::{HttpRequest, HttpResponse, ContentType};
use axum::http::StatusCode;

// Create request from Axum request
let request = HttpRequest::from_axum_request(axum_request).await;

// Access request properties
let method = request.method;  // HttpMethod enum
let path = &request.path;
let query_param = request.get_query_param("id");
let header = request.get_header("authorization");

// Create responses
let json_response = HttpResponse::json(StatusCode::OK, &data)?;
let text_response = HttpResponse::text(StatusCode::OK, "Hello");
let binary_response = HttpResponse::binary(StatusCode::OK, bytes);
let error_response = HttpResponse::error(StatusCode::BAD_REQUEST, "Error");
```

### Using HttpRequestProducer

```rust
use streamweave::http_server::{HttpRequestProducer, HttpRequestProducerConfig};

let producer = HttpRequestProducer::from_axum_request(
    axum_request,
    HttpRequestProducerConfig::default()
        .with_extract_body(true)
        .with_parse_json(true)
        .with_extract_query_params(true)
        .with_extract_path_params(true),
).await;

let mut stream = producer.produce();
while let Some(request) = stream.next().await {
    // Process request
}
```

### Using HttpResponseConsumer

```rust
use streamweave::http_server::{HttpResponseConsumer, HttpResponseConsumerConfig};

let consumer = HttpResponseConsumer::with_config(
    HttpResponseConsumerConfig::default()
        .with_stream_response(false)
        .with_merge_responses(true),
);

// ... consume stream ...

let axum_response = consumer.get_response().await;
```

### Creating Route Handlers

```rust
use streamweave::http_server::{create_simple_handler, create_pipeline_handler};
use streamweave::prelude::*;
use axum::{Router, routing::get, routing::post};

// Simple handler
let app = Router::new()
    .route("/api/echo", get(create_simple_handler(|req: HttpRequest| {
        HttpResponse::text(StatusCode::OK, &format!("Echo: {}", req.path))
    })));

// Pipeline handler with transformer
let app = Router::new()
    .route("/api/process", post(create_pipeline_handler(|| {
        MapTransformer::new(|req: HttpRequest| {
            // Transform request to response
            HttpResponse::json(StatusCode::OK, &process_request(req))
        })
    })));
```

## Architecture

The example demonstrates the following architecture:

```
HTTP Request (Axum)
    ↓
HttpRequestProducer (Task 16.2)
    ↓
StreamWeave Pipeline
    ↓ (optional transformers)
    ↓
HttpResponseConsumer (Task 16.3)
    ↓
HTTP Response (Axum)
```

## Error Handling

The example demonstrates error handling at multiple levels:

1. **Request Parsing Errors**: Handled by HttpRequestProducer
2. **Pipeline Errors**: Converted to HTTP error responses
3. **Response Errors**: Handled by HttpResponseConsumer

## Best Practices

1. **Use Appropriate Content Types**: Set correct Content-Type headers for responses
2. **Handle Errors Gracefully**: Convert pipeline errors to appropriate HTTP status codes
3. **Extract Metadata Early**: Use HttpRequestProducer to extract all needed metadata
4. **Use Pipeline Handlers**: For complex processing, use `create_pipeline_handler` with transformers
5. **Set Response Headers**: Use `add_header` or `set_header` for custom headers

## Additional Features

### Task 16.5: Streaming Request Bodies

The example includes an endpoint `/api/stream-upload` that demonstrates streaming large request bodies:

```bash
curl -X POST http://127.0.0.1:3000/api/stream-upload \
  -H "Content-Type: application/octet-stream" \
  --data-binary @large-file.bin
```

### Task 16.6: Streaming Response Bodies

The example includes an endpoint `/api/stream-download` that demonstrates streaming large response bodies:

```bash
curl http://127.0.0.1:3000/api/stream-download
```

### Task 16.7: Middleware Support

The example applies CORS and logging middleware to all routes:

```rust
let app = app
    .layer(cors_layer())
    .layer(logging_layer());
```

### Task 16.8: Comprehensive Error Handling

The example includes multiple error handling endpoints:

- `GET /api/error/validation` - Demonstrates validation errors (400)
- `GET /api/error/not-found` - Demonstrates not found errors (404)
- `GET /api/error/custom` - Demonstrates custom error responses (409)

### Task 16.9: Complete REST Microservice

This example serves as a complete REST microservice demonstrating all HTTP server capabilities including:
- Multiple REST endpoints with various HTTP methods
- Request/response body streaming
- Comprehensive error handling
- Middleware integration
- Pipeline-based processing

## Troubleshooting

### Server Won't Start

- Ensure port 3000 is not in use
- Check that `http-server` feature is enabled
- Verify Tokio runtime is available

### Requests Fail

- Check that Content-Type headers are set correctly
- Verify request body format matches expected content type
- Check server logs for error messages

### Responses Not Working

- Ensure HttpResponse is properly constructed
- Check status codes are appropriate
- Verify Content-Type headers are set

