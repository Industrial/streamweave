# HTTP Router Transformer Example

This example demonstrates the **HTTP Router Transformer**, a core component of StreamWeave's HTTP server architecture that handles request routing and dispatching.

## Overview

The HTTP Router Transformer replaces traditional web framework routers (like Axum's router) by providing a streaming-first, composable approach to HTTP request handling within StreamWeave pipelines.

## Key Features

### ✅ Route Pattern Matching
- **Path Parameters**: Support for dynamic path segments like `/users/{id}`
- **Query Parameters**: Automatic parsing of query strings
- **HTTP Methods**: Full support for GET, POST, PUT, DELETE, etc.
- **Priority-based Matching**: More specific routes take precedence
- **Regex Support**: Advanced pattern matching capabilities

### ✅ Handler System
- **Trait-based Handlers**: Implement `HttpHandler` trait for custom logic
- **Async Support**: Full async/await compatibility
- **Type Safety**: Compile-time route validation
- **Error Handling**: Built-in error handling and fallback support

### ✅ Middleware Support
- **Request Processing**: Pre-process requests before handlers
- **Response Processing**: Post-process responses after handlers
- **Composable**: Chain multiple middleware components
- **Error Recovery**: Graceful error handling throughout the pipeline

## Architecture

```
HTTP Request → Router Transformer → Handler → Response
                    ↓
              Middleware Chain
```

## Usage Example

```rust
use streamweave::{
    structs::{
        http::{HttpHandler, RoutePattern, StreamWeaveHttpRequestChunk, StreamWeaveHttpResponse},
        transformers::http_router::HttpRouterTransformer,
    },
    traits::transformer::TransformerConfig,
};
use http::Method;
use std::sync::Arc;

// Define a custom handler
struct MyHandler;

#[async_trait::async_trait]
impl HttpHandler for MyHandler {
    async fn handle(&self, request: StreamWeaveHttpRequestChunk) -> StreamWeaveHttpResponse {
        let body = format!("Hello from {} {}", request.method, request.path());
        StreamWeaveHttpResponse::ok(body.into())
    }
}

// Create route patterns
let user_route = RoutePattern::new(Method::GET, "/users/{id}")?;
let api_route = RoutePattern::new(Method::GET, "/api/*")?;

// Build the router
let router = HttpRouterTransformer::new()
    .add_route(user_route, "user_handler".to_string(), Arc::new(MyHandler))
    .add_route(api_route, "api_handler".to_string(), Arc::new(MyHandler))
    .with_fallback_handler(Arc::new(NotFoundHandler))
    .with_config(TransformerConfig::default().with_name("my_router".to_string()));

// Find matching routes
if let Some((handler_id, path_params)) = router.find_route(&Method::GET, "/users/123") {
    println!("Found route: {} with params: {:?}", handler_id, path_params);
}
```

## Route Pattern Syntax

### Static Routes
```rust
RoutePattern::new(Method::GET, "/api/health")?;
RoutePattern::new(Method::POST, "/api/users")?;
```

### Dynamic Routes with Parameters
```rust
RoutePattern::new(Method::GET, "/users/{id}")?;                    // /users/123
RoutePattern::new(Method::GET, "/users/{id}/posts/{post_id}")?;    // /users/123/posts/456
RoutePattern::new(Method::PUT, "/api/v1/users/{user_id}")?;        // /api/v1/users/789
```

### Wildcard Routes
```rust
RoutePattern::new(Method::GET, "/api/*")?;                         // /api/anything/here
RoutePattern::new(Method::GET, "/static/**")?;                     // /static/files/images/logo.png
```

## Handler Implementation

Handlers must implement the `HttpHandler` trait:

```rust
#[async_trait::async_trait]
impl HttpHandler for MyHandler {
    async fn handle(&self, request: StreamWeaveHttpRequestChunk) -> StreamWeaveHttpResponse {
        // Access request data
        let method = &request.method;
        let path = request.path();
        let user_id = request.path_param("id");
        let query_param = request.query_param("filter");
        
        // Process the request
        let response_body = process_request(method, path, user_id, query_param);
        
        // Return response
        StreamWeaveHttpResponse::ok(response_body.into())
            .with_content_type("application/json")
    }
}
```

## Response Building

The `StreamWeaveHttpResponse` provides convenient methods for building responses:

```rust
// Basic responses
StreamWeaveHttpResponse::ok(body.into())
StreamWeaveHttpResponse::not_found(body.into())
StreamWeaveHttpResponse::internal_server_error(body.into())

// With headers
response.with_content_type("application/json")
response.with_header("x-custom", "value")

// With trailers
response.with_trailers(trailer_headers)
```

## Error Handling

The router provides comprehensive error handling:

```rust
// Route conflicts are detected at build time
let result = router.add_route(conflicting_pattern, "handler".to_string(), handler);
match result {
    Ok(router) => println!("Route added successfully"),
    Err(RouterError::RouteConflict { existing, new }) => {
        println!("Route conflict: {} vs {}", existing.path, new.path);
    }
}

// Fallback handler for unmatched routes
router.with_fallback_handler(Arc::new(NotFoundHandler));
```

## Integration with StreamWeave Pipelines

The HTTP Router Transformer integrates seamlessly with StreamWeave's pipeline system:

```rust
use streamweave::{PipelineBuilder, impls::transformers::http_router::HttpRouterTransformer};

let pipeline = PipelineBuilder::new()
    .producer(http_request_producer)
    .transformer(router_transformer)
    .consumer(http_response_consumer)
    .run()
    .await?;
```

## Performance Characteristics

- **O(n) Route Matching**: Linear search through registered routes
- **Priority-based Selection**: More specific routes are matched first
- **Memory Efficient**: Minimal overhead per route
- **Concurrent Safe**: Thread-safe for concurrent request handling

## Testing

The example includes comprehensive tests covering:

- Route pattern creation and validation
- Path parameter extraction
- Query parameter parsing
- Route matching with different priorities
- Error handling scenarios
- Fallback handler execution

Run the example:

```bash
cargo run --example 04_http_router
```

## Future Enhancements

- **Middleware Pipeline**: Ordered middleware execution
- **Route Caching**: Performance optimization for frequently accessed routes
- **Metrics Collection**: Built-in performance monitoring
- **Route Groups**: Organize related routes together
- **Conditional Routing**: Route based on headers, query params, etc.

## Related Components

- **HTTP Server Producer**: Accepts incoming HTTP connections
- **HTTP Response Consumer**: Sends responses back to clients
- **HTTP Middleware System**: Request/response processing pipeline
- **HTTP Error Handler**: Converts errors to appropriate HTTP responses

This HTTP Router Transformer is a foundational component for building high-performance, streaming HTTP servers with StreamWeave.
