# streamweave-http-server

[![Crates.io](https://img.shields.io/crates/v/streamweave-http-server.svg)](https://crates.io/crates/streamweave-http-server)
[![Documentation](https://docs.rs/streamweave-http-server/badge.svg)](https://docs.rs/streamweave-http-server)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**HTTP server integration for StreamWeave**  
*Build HTTP APIs and servers using StreamWeave pipelines and graphs.*

The `streamweave-http-server` package provides HTTP server integration for StreamWeave. It enables building HTTP APIs using StreamWeave pipelines and graphs, with support for Axum integration, middleware, request/response handling, and path-based routing.

## ✨ Key Features

- **HttpGraphServer**: Long-lived graph-based HTTP server
- **HttpRequestProducer**: Convert HTTP requests to stream items
- **HttpResponseConsumer**: Convert stream items to HTTP responses
- **Axum Integration**: Full Axum framework integration
- **Middleware Support**: CORS, logging, authentication middleware
- **Path Routing**: Path-based routing with transformers
- **Request/Response Correlation**: Automatic request/response matching
- **Streaming Support**: Support for streaming request/response bodies

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-http-server = { version = "0.3.0", features = ["http-server"] }
```

## 🚀 Quick Start

### Simple HTTP Handler

```rust
use streamweave_http_server::create_simple_handler;
use streamweave_http_server::{HttpRequest, HttpResponse};
use axum::{Router, routing::get};
use axum::http::StatusCode;

let app = Router::new()
    .route("/api/echo", get(create_simple_handler(|req: HttpRequest| {
        HttpResponse::text(StatusCode::OK, &format!("Echo: {}", req.path))
    })));
```

### HTTP Graph Server

```rust
use streamweave_http_server::{HttpGraphServer, HttpGraphServerConfig};
use streamweave_graph::{GraphBuilder, Graph};
use axum::{Router, routing::any};

// Build graph with HTTP producer and consumer
let graph = GraphBuilder::new()
    .add_producer(/* HTTP request producer */)
    .add_transformer(/* transform requests */)
    .add_consumer(/* HTTP response consumer */)
    .build()?;

// Create HTTP graph server
let config = HttpGraphServerConfig::default()
    .with_request_timeout(Duration::from_secs(30));

let (server, request_receiver) = HttpGraphServer::new(graph, config).await?;
server.start().await?;

// Create Axum handler
let handler = server.create_handler();

let app = Router::new()
    .route("/*path", any(handler));
```

## 📖 API Overview

### HttpGraphServer

Long-lived graph-based HTTP server:

```rust
pub struct HttpGraphServer {
    executor: Arc<RwLock<GraphExecutor>>,
    request_sender: mpsc::Sender<Request>,
    response_consumer: Arc<HttpResponseCorrelationConsumer>,
    config: HttpGraphServerConfig,
}
```

**Key Methods:**
- `new(graph, config)` - Create server from graph
- `start()` - Start graph executor
- `stop()` - Stop graph executor
- `handle_request(request)` - Handle HTTP request
- `create_handler()` - Create Axum handler function

### HttpRequestProducer

Converts HTTP requests to stream items:

```rust
pub struct HttpRequestProducer {
    pub config: ProducerConfig<HttpRequest>,
    pub request_producer_config: HttpRequestProducerConfig,
}
```

**Key Methods:**
- `from_axum_request(request, config)` - Create from Axum request
- `with_extract_body(extract)` - Configure body extraction
- `with_parse_json(parse)` - Configure JSON parsing
- `produce()` - Generate stream from HTTP request

### HttpResponseConsumer

Converts stream items to HTTP responses:

```rust
pub struct HttpResponseConsumer {
    pub config: ConsumerConfig<HttpResponse>,
    pub response_consumer_config: HttpResponseConsumerConfig,
}
```

**Key Methods:**
- `new(config)` - Create consumer with configuration
- `with_stream_response(stream)` - Configure streaming responses
- `with_merge_responses(merge)` - Configure response merging
- `consume(stream)` - Convert stream to HTTP response

## 📚 Usage Examples

### Path-Based Routing

Use path routing transformers:

```rust
use streamweave_http_server::transformers::PathRouterTransformer;
use streamweave_graph::GraphBuilder;

let graph = GraphBuilder::new()
    .add_producer(/* HTTP request producer */)
    .add_transformer(PathRouterTransformer::new()
        .route("/api/users", /* user handler */)
        .route("/api/posts", /* post handler */)
        .default(/* default handler */))
    .add_consumer(/* HTTP response consumer */)
    .build()?;
```

### Middleware Support

Add middleware layers:

```rust
use streamweave_http_server::middleware;
use axum::Router;

let app = Router::new()
    .layer(middleware::cors_layer())
    .layer(middleware::logging_layer())
    .layer(middleware::cors_with_origins(vec!["http://localhost:3000"]));
```

### Request Configuration

Configure request extraction:

```rust
use streamweave_http_server::{HttpRequestProducer, HttpRequestProducerConfig};

let config = HttpRequestProducerConfig::default()
    .with_extract_body(true)
    .with_max_body_size(Some(10 * 1024 * 1024))  // 10MB limit
    .with_parse_json(true)
    .with_extract_query_params(true)
    .with_extract_path_params(true)
    .with_stream_body(false);

let producer = HttpRequestProducer::from_axum_request(request, config).await;
```

### Response Configuration

Configure response handling:

```rust
use streamweave_http_server::{HttpResponseConsumer, HttpResponseConsumerConfig};
use axum::http::StatusCode;

let config = HttpResponseConsumerConfig::default()
    .with_stream_response(false)
    .with_max_items(None)
    .with_merge_responses(false)
    .with_default_status(StatusCode::OK);

let consumer = HttpResponseConsumer::new(config);
```

### Streaming Responses

Enable streaming responses:

```rust
use streamweave_http_server::{HttpResponseConsumer, HttpResponseConsumerConfig};

let config = HttpResponseConsumerConfig::default()
    .with_stream_response(true)
    .with_max_items(Some(100));  // Stream in batches of 100

let consumer = HttpResponseConsumer::new(config);
```

### Error Handling

Handle errors in HTTP handlers:

```rust
use streamweave_http_server::{HttpRequest, HttpResponse};
use streamweave_error::ErrorStrategy;
use axum::http::StatusCode;

let handler = create_simple_handler(|req: HttpRequest| {
    match process_request(&req) {
        Ok(result) => HttpResponse::json(StatusCode::OK, &result),
        Err(e) => HttpResponse::text(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
});
```

## 🏗️ Architecture

HTTP server integration flow:

```text
HTTP Request ──> HttpRequestProducer ──> Stream<HttpRequest> ──> Transformer ──> Stream<HttpResponse> ──> HttpResponseConsumer ──> HTTP Response
```

**HTTP Flow:**
1. HTTP request arrives at Axum router
2. HttpRequestProducer extracts request data
3. HttpRequest items flow through transformers
4. HttpResponseConsumer converts responses
5. HTTP response sent back to client
6. Request/response correlation matches responses to requests

## 🔧 Configuration

### HttpGraphServerConfig

- **request_timeout**: Request timeout duration (default: 30 seconds)
- **request_channel_buffer**: Channel buffer size for requests (default: 100)

### HttpRequestProducerConfig

- **extract_body**: Whether to extract request body
- **max_body_size**: Maximum body size to extract
- **parse_json**: Whether to parse JSON body automatically
- **extract_query_params**: Whether to extract query parameters
- **extract_path_params**: Whether to extract path parameters
- **stream_body**: Whether to stream body in chunks
- **chunk_size**: Chunk size for streaming

### HttpResponseConsumerConfig

- **stream_response**: Whether to stream responses
- **max_items**: Maximum items to collect before sending
- **merge_responses**: Whether to merge multiple responses
- **default_status**: Default status code

## 🔍 Error Handling

HTTP errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let producer = HttpRequestProducer::from_axum_request(request, config)
    .await
    .with_error_strategy(ErrorStrategy::Skip);
```

## ⚡ Performance Considerations

- **Request Timeout**: Configure appropriate timeouts
- **Body Streaming**: Use streaming for large request bodies
- **Response Batching**: Batch responses for better performance
- **Connection Pooling**: Reuse connections when possible

## 📝 Examples

For more examples, see:
- [HTTP Server Integration Example](https://github.com/Industrial/streamweave/tree/main/examples/http_server_integration)
- [HTTP Graph Server Example](https://github.com/Industrial/streamweave/tree/main/examples/http_graph_server)
- [HTTP-Specific Examples](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-http-server` depends on:

- `streamweave` - Core traits
- `streamweave-graph` - Graph execution
- `streamweave-pipeline` - Pipeline execution
- `streamweave-message` - Message envelopes
- `streamweave-error` - Error handling
- `axum` - HTTP framework
- `tower` - Middleware framework
- `tower-http` - HTTP middleware
- `tokio` - Async runtime
- `futures` - Stream utilities
- `serde` - Serialization support

## 🎯 Use Cases

HTTP server integration is used for:

1. **REST APIs**: Build REST APIs with StreamWeave
2. **Graph-Based Processing**: Process requests through graphs
3. **Microservices**: Build microservices with HTTP interfaces
4. **Request Transformation**: Transform requests through pipelines
5. **API Gateways**: Build API gateways with routing

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-http-server)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/http-server)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-graph](../graph/README.md) - Graph execution
- [streamweave-pipeline](../pipeline/README.md) - Pipeline execution
- [streamweave-error](../error/README.md) - Error handling

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

