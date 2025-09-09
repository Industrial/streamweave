# HTTP Router Transformer Example

This example demonstrates the **HTTP Router Transformer** with **HTTP Response Builder** integration, showcasing StreamWeave's streaming-first approach to HTTP request routing and response generation.

## Overview

The HTTP Router Transformer replaces traditional web framework routers by providing a streaming-first, composable approach to HTTP request handling within StreamWeave pipelines. This example now features full integration with the HTTP Response Builder for structured, secure response generation.

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

### 🆕 HTTP Response Builder Integration
- **HtmlResponseBuilder**: Rich HTML responses with styling
- **JsonResponseBuilder**: Structured API responses
- **Convenience Functions**: Quick response creation with `responses` module
- **Security Headers**: Automatic security header injection
- **CORS Support**: Cross-origin resource sharing configuration
- **Content-Type Management**: Automatic content type handling

## Architecture

```
HTTP Request → Router Transformer → Handler → Response Builder → HTTP Response
                    ↓                              ↓
              Middleware Chain              Security Headers + CORS
```

## Usage Example

```rust
use streamweave::{
    http::{HttpHandler, RoutePattern, StreamWeaveHttpRequestChunk, StreamWeaveHttpResponse},
    transformers::{
        http_router::http_router_transformer::HttpRouterTransformer,
        http_response_builder::{
            response_data::ResponseData,
            builder_utils::{JsonResponseBuilder, HtmlResponseBuilder, responses},
            http_response_builder_transformer::HttpResponseBuilderTransformer,
        },
    },
    transformer::TransformerConfig,
};
use http::Method;
use std::sync::Arc;

// Define a custom handler using HTTP Response Builder
struct MyHandler;

#[async_trait::async_trait]
impl HttpHandler for MyHandler {
    async fn handle(&self, request: StreamWeaveHttpRequestChunk) -> StreamWeaveHttpResponse {
        // Use JsonResponseBuilder for structured API responses
        let response_data = JsonResponseBuilder::new()
            .with_status(http::StatusCode::OK)
            .field("message", "Hello from StreamWeave!")
            .field("method", request.method.as_str())
            .field("path", request.path())
            .field("timestamp", &chrono::Utc::now().to_rfc3339())
            .header("x-powered-by", "StreamWeave")
            .build();

        // Convert to HTTP response with security headers
        let response_builder = HttpResponseBuilderTransformer::new()
            .with_security_headers(true)
            .with_cors_headers(true);
        
        response_builder.build_response(response_data).await
            .unwrap_or_else(|_| StreamWeaveHttpResponse::internal_server_error("Failed to build response".into()))
    }
}
```

## Response Builder Features

### HTML Responses
```rust
let response_data = HtmlResponseBuilder::new()
    .with_status(http::StatusCode::OK)
    .content(&format!(
        r#"<!DOCTYPE html>
        <html>
        <head><title>StreamWeave</title></head>
        <body>
            <h1>Hello from {}</h1>
            <p>Path: {}</p>
        </body>
        </html>"#,
        request.method, request.path()
    ))
    .header("x-powered-by", "StreamWeave")
    .build();
```

### JSON API Responses
```rust
let response_data = JsonResponseBuilder::new()
    .with_status(http::StatusCode::OK)
    .field("user_id", &user_id)
    .field("name", &format!("User {}", user_id))
    .field("email", &format!("user{}@example.com", user_id))
    .header("x-api-version", "v1")
    .build();
```

### Convenience Functions
```rust
// Quick JSON responses
let response_data = responses::ok_json(serde_json::json!({
    "message": "API response",
    "data": some_data
}));

// Quick error responses
let response_data = responses::not_found_json(serde_json::json!({
    "error": "Resource not found",
    "path": request.path()
}));
```

## Security Features

The HTTP Response Builder automatically adds security headers when enabled:

```rust
let response_builder = HttpResponseBuilderTransformer::new()
    .with_security_headers(true)  // Adds X-Content-Type-Options, X-Frame-Options, etc.
    .with_cors_headers(true);     // Adds CORS headers for cross-origin requests
```

Security headers include:
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `X-XSS-Protection: 1; mode=block`
- `Strict-Transport-Security: max-age=31536000; includeSubDomains`
- `Content-Security-Policy: default-src 'self'`

## CORS Configuration

```rust
use streamweave::transformers::http_response_builder::http_response_builder_transformer::CorsConfig;

let cors_config = CorsConfig {
    allowed_origins: vec!["https://example.com".to_string()],
    allowed_methods: vec!["GET".to_string(), "POST".to_string()],
    allowed_headers: vec!["content-type".to_string()],
    allow_credentials: true,
    max_age: Some(3600),
};

let response_builder = HttpResponseBuilderTransformer::new()
    .with_cors_headers(true)
    .with_cors_config(cors_config);
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

## Example Handlers

The example includes four different handler types:

1. **HelloHandler**: Returns rich HTML responses using `HtmlResponseBuilder`
2. **UserHandler**: Returns structured JSON using `JsonResponseBuilder`
3. **ApiHandler**: Uses convenience functions from `responses` module
4. **NotFoundHandler**: Demonstrates error responses with proper HTTP status codes

## Testing

Run the example to see all features in action:

```bash
cargo run --example 04_http_router
```

The example will test:
- HTML responses with styling and security headers
- JSON API responses with structured data
- Error responses with proper status codes
- CORS headers for cross-origin requests
- Security headers for protection

## Integration with StreamWeave Pipelines

```rust
use streamweave::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(http_request_producer)
    .transformer(router_transformer)
    .transformer(response_builder_transformer)  // Optional additional processing
    .consumer(http_response_consumer)
    .run()
    .await?;
```

## Performance Benefits

- **Streaming Architecture**: No buffering, immediate response generation
- **Security by Default**: Automatic security headers without performance overhead
- **Type Safety**: Compile-time validation of response structures
- **Memory Efficient**: Minimal allocations for response building

## Related Components

- **HTTP Response Builder Transformer**: Core response building functionality
- **HTTP Server Producer**: Accepts incoming HTTP connections
- **HTTP Response Consumer**: Sends responses back to clients
- **HTTP Middleware System**: Request/response processing pipeline

This example demonstrates how StreamWeave's HTTP Router Transformer and HTTP Response Builder work together to create secure, structured HTTP responses in a streaming architecture.