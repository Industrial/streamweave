# StreamWeave HTTP Server Core Components TODO

This document outlines the components needed to make StreamWeave the primary architectural component of an HTTP server, replacing the current secondary role where it's only used within Axum route handlers.

## Current State
- StreamWeave is currently used as a secondary component within Axum route handlers
- Architecture: `Axum Router → Route Handler → StreamWeave Pipeline → Response`
- Inefficient because Axum handles all HTTP protocol details and StreamWeave only processes request bodies
- Pipeline is created/destroyed per request

## Target Architecture
- StreamWeave becomes the primary component handling the entire HTTP request/response cycle
- Architecture: `StreamWeave Pipeline (HTTP Server) → Route Handlers → Response`
- True streaming throughout the entire pipeline
- Built-in backpressure, monitoring, and composability

---

## 1. HTTP Router Transformer

**Priority: HIGH** | **Estimated Effort: 3-4 days**

### Context
A transformer that handles HTTP routing logic, replacing Axum's router functionality. This is the core component that will determine which handler pipeline to execute based on the request path and method.

### Requirements
- Route requests to appropriate handler pipelines based on path patterns and HTTP methods
- Support path parameters (e.g., `/users/{id}`)
- Support query parameters
- Support wildcard and regex patterns
- Maintain route metadata for downstream transformers
- Handle 404 responses for unmatched routes
- Support middleware ordering and execution

### Implementation Details
```rust
pub struct HttpRouterTransformer {
    routes: HashMap<RoutePattern, Box<dyn HttpHandler>>,
    fallback_handler: Option<Box<dyn HttpHandler>>,
    config: TransformerConfig<StreamWeaveHttpRequestChunk>,
}

pub struct RoutePattern {
    method: Method,
    path: String,
    path_params: Vec<String>, // e.g., ["id", "name"]
}

pub trait HttpHandler: Send + Sync {
    async fn handle(&self, request: StreamWeaveHttpRequestChunk) -> StreamWeaveHttpResponseChunk;
}
```

### Inline Tests Required (100% Coverage)
- Route matching with exact paths
- Route matching with path parameters
- Route matching with query parameters
- Route matching with different HTTP methods
- 404 handling for unmatched routes
- Fallback handler execution
- Route priority and precedence
- Performance with large number of routes
- Error handling for malformed routes
- Concurrent route registration and matching

---

## 2. HTTP Middleware System

**Priority: HIGH** | **Estimated Effort: 2-3 days**

### Context
A collection of transformers that implement common HTTP middleware patterns, allowing for composable request/response processing.

### Requirements
- Authentication middleware
- CORS middleware
- Logging middleware
- Compression middleware
- Rate limiting middleware
- Request validation middleware
- Response transformation middleware

### Implementation Details
```rust
// Authentication middleware
pub struct AuthTransformer {
    auth_strategies: Vec<Box<dyn AuthStrategy>>,
    config: TransformerConfig<StreamWeaveHttpRequestChunk>,
}

// CORS middleware
pub struct CorsTransformer {
    allowed_origins: Vec<String>,
    allowed_methods: Vec<Method>,
    allowed_headers: Vec<String>,
    config: TransformerConfig<StreamWeaveHttpRequestChunk>,
}

// Logging middleware
pub struct LoggingTransformer {
    log_level: LogLevel,
    include_body: bool,
    config: TransformerConfig<StreamWeaveHttpRequestChunk>,
}
```

### Inline Tests Required (100% Coverage)
- Authentication success and failure scenarios
- CORS preflight and actual request handling
- Request/response logging with different log levels
- Compression with different algorithms and content types
- Rate limiting with various strategies
- Middleware ordering and execution
- Error handling and fallback behavior
- Performance impact measurement
- Configuration validation
- Concurrent request handling

---

## 3. HTTP Response Builder Transformer

**Priority: HIGH** | **Estimated Effort: 2-3 days**

### Context
A transformer that constructs proper HTTP responses from processed data, handling status codes, headers, and body formatting.

### Requirements
- Convert processed data to HTTP response format
- Handle different content types (JSON, HTML, plain text, binary)
- Set appropriate status codes
- Manage response headers
- Support streaming responses
- Handle error responses
- Support chunked transfer encoding

### Implementation Details
```rust
pub struct HttpResponseBuilderTransformer {
    default_content_type: String,
    compression_enabled: bool,
    config: TransformerConfig<ResponseData>,
}

pub enum ResponseData {
    Success { status: StatusCode, body: Bytes, headers: HeaderMap },
    Error { status: StatusCode, message: String },
    Stream { status: StatusCode, stream: Box<dyn Stream<Item = Bytes>> },
}
```

### Inline Tests Required (100% Coverage)
- Response building with different status codes
- Header management and validation
- Content type detection and setting
- Streaming response handling
- Error response formatting
- Chunked transfer encoding
- Compression integration
- Large response handling
- Concurrent response building
- Memory usage optimization

---

## 4. HTTP Server Producer

**Priority: HIGH** | **Estimated Effort: 3-4 days**

### Context
A producer that accepts incoming HTTP connections and produces HTTP request streams, replacing the need for Axum's server functionality.

### Requirements
- Accept incoming TCP connections
- Parse HTTP requests
- Handle HTTP/1.1 and HTTP/2 protocols
- Manage connection lifecycle
- Support keep-alive connections
- Handle connection timeouts
- Support TLS/SSL
- Graceful shutdown

### Implementation Details
```rust
pub struct HttpServerProducer {
    listener: TcpListener,
    max_connections: usize,
    connection_timeout: Duration,
    keep_alive_timeout: Duration,
    config: ProducerConfig<StreamWeaveHttpRequestChunk>,
}

impl Producer for HttpServerProducer {
    type Output = StreamWeaveHttpRequestChunk;
    type OutputStream = Pin<Box<dyn Stream<Item = StreamWeaveHttpRequestChunk> + Send>>;
}
```

### Inline Tests Required (100% Coverage)
- Connection acceptance and management
- HTTP request parsing (various formats)
- Keep-alive connection handling
- Connection timeout management
- Graceful shutdown
- Error handling for malformed requests
- Concurrent connection handling
- Memory usage with many connections
- TLS/SSL integration
- Performance under load

---

## 5. HTTP Response Consumer

**Priority: HIGH** | **Estimated Effort: 2-3 days**

### Context
An enhanced consumer that can send HTTP responses back to clients, handling the complete response lifecycle.

### Requirements
- Send HTTP responses to clients
- Handle streaming responses
- Manage connection state
- Support chunked transfer encoding
- Handle response timeouts
- Support compression
- Error handling and recovery

### Implementation Details
```rust
pub struct HttpResponseConsumer {
    response_sender: tokio::sync::mpsc::Sender<ResponseChunk>,
    connection_manager: Arc<ConnectionManager>,
    config: ConsumerConfig<ResponseChunk>,
}

pub enum ResponseChunk {
    Header(StatusCode, HeaderMap),
    Body(Bytes),
    End,
    Error(StatusCode, String),
}
```

### Inline Tests Required (100% Coverage)
- Response sending with different formats
- Streaming response handling
- Connection state management
- Chunked transfer encoding
- Response timeout handling
- Error recovery and retry logic
- Compression support
- Concurrent response handling
- Memory management
- Performance optimization

---

## 6. Enhanced HTTP Types

**Priority: MEDIUM** | **Estimated Effort: 1-2 days**

### Context
More sophisticated HTTP types for better integration and easier manipulation within StreamWeave pipelines.

### Requirements
- Rich request representation with path/query parameters
- Enhanced response types
- Better error handling types
- Streaming body support
- Header manipulation utilities

### Implementation Details
```rust
pub struct StreamWeaveHttpRequest {
    pub method: Method,
    pub uri: Uri,
    pub headers: HeaderMap,
    pub body: BodyStream,
    pub path_params: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub connection_info: ConnectionInfo,
}

pub struct StreamWeaveHttpResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: BodyStream,
    pub trailers: Option<HeaderMap>,
}

pub struct ConnectionInfo {
    pub remote_addr: SocketAddr,
    pub local_addr: SocketAddr,
    pub protocol: HttpVersion,
    pub tls_info: Option<TlsInfo>,
}
```

### Inline Tests Required (100% Coverage)
- Request parsing and validation
- Parameter extraction (path and query)
- Header manipulation
- Body streaming
- Response construction
- Error type handling
- Serialization/deserialization
- Memory efficiency
- Type safety validation
- Integration with existing types

---

## 7. Connection Management Transformer

**Priority: MEDIUM** | **Estimated Effort: 2-3 days**

### Context
A transformer to handle HTTP connection lifecycle, including keep-alive, pooling, and cleanup.

### Requirements
- Manage keep-alive connections
- Connection pooling
- Timeout management
- Connection cleanup
- Resource monitoring
- Load balancing support

### Implementation Details
```rust
pub struct HttpConnectionManager {
    max_connections: usize,
    keep_alive_timeout: Duration,
    connection_pool: Arc<ConnectionPool>,
    config: TransformerConfig<StreamWeaveHttpRequestChunk>,
}

pub struct ConnectionPool {
    active_connections: HashMap<ConnectionId, ConnectionInfo>,
    idle_connections: VecDeque<ConnectionId>,
    max_idle: usize,
}
```

### Inline Tests Required (100% Coverage)
- Connection lifecycle management
- Keep-alive timeout handling
- Connection pooling efficiency
- Resource cleanup
- Memory leak prevention
- Concurrent connection handling
- Load balancing algorithms
- Performance monitoring
- Error recovery
- Configuration validation

---

## 8. HTTP Error Handling System

**Priority: MEDIUM** | **Estimated Effort: 1-2 days**

### Context
HTTP-specific error handling transformers that convert StreamWeave errors to appropriate HTTP status codes and responses.

### Requirements
- Convert internal errors to HTTP status codes
- Provide user-friendly error messages
- Support custom error pages
- Error logging and monitoring
- Error recovery strategies

### Implementation Details
```rust
pub struct HttpErrorHandler {
    error_mappings: HashMap<ErrorType, StatusCode>,
    custom_error_pages: HashMap<StatusCode, String>,
    config: TransformerConfig<StreamError>,
}

pub enum ErrorType {
    NotFound,
    Unauthorized,
    Forbidden,
    InternalServerError,
    BadRequest,
    Timeout,
    RateLimited,
}
```

### Inline Tests Required (100% Coverage)
- Error type to status code mapping
- Custom error page rendering
- Error logging functionality
- Error recovery strategies
- User-friendly error messages
- Error context preservation
- Performance impact measurement
- Error rate monitoring
- Fallback error handling
- Error response formatting

---

## 9. HTTP Pipeline Builder

**Priority: LOW** | **Estimated Effort: 1-2 days**

### Context
A specialized pipeline builder for HTTP servers that provides convenient methods for common HTTP server patterns.

### Requirements
- Fluent API for HTTP server configuration
- Route registration helpers
- Middleware composition utilities
- Error handling configuration
- Performance tuning options

### Implementation Details
```rust
pub struct HttpServerBuilder {
    routes: Vec<RouteDefinition>,
    middleware: Vec<Box<dyn HttpMiddleware>>,
    error_handler: Option<Box<dyn HttpErrorHandler>>,
    config: HttpServerConfig,
}

impl HttpServerBuilder {
    pub fn route(mut self, pattern: &str, handler: impl HttpHandler) -> Self;
    pub fn middleware(mut self, middleware: impl HttpMiddleware) -> Self;
    pub fn error_handler(mut self, handler: impl HttpErrorHandler) -> Self;
    pub fn build(self) -> HttpServerPipeline;
}
```

### Inline Tests Required (100% Coverage)
- Route registration and validation
- Middleware composition
- Error handler configuration
- Configuration validation
- Pipeline building
- Performance optimization
- Memory usage
- Type safety
- Documentation examples
- Integration testing

---

## 10. HTTP Server Integration Tests

**Priority: HIGH** | **Estimated Effort: 2-3 days**

### Context
Comprehensive integration tests that verify the entire HTTP server pipeline works correctly with all components.

### Requirements
- End-to-end HTTP request/response testing
- Performance benchmarking
- Load testing
- Error scenario testing
- Compatibility testing with HTTP clients
- Memory leak detection

### Implementation Details
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_complete_http_server_pipeline() {
        // Test complete request/response cycle
    }
    
    #[tokio::test]
    async fn test_concurrent_requests() {
        // Test handling multiple concurrent requests
    }
    
    #[tokio::test]
    async fn test_error_handling() {
        // Test various error scenarios
    }
}
```

### Inline Tests Required (100% Coverage)
- Complete request/response cycles
- Concurrent request handling
- Error scenario coverage
- Performance benchmarks
- Memory usage validation
- Connection management
- Middleware execution order
- Route matching accuracy
- Response formatting
- Client compatibility

---

## Implementation Priority

1. **Phase 1 (Core)**: HTTP Router Transformer, HTTP Response Builder, HTTP Server Producer, HTTP Response Consumer
2. **Phase 2 (Middleware)**: HTTP Middleware System, Enhanced HTTP Types
3. **Phase 3 (Management)**: Connection Management, Error Handling
4. **Phase 4 (Polish)**: HTTP Pipeline Builder, Integration Tests

## Success Criteria

- StreamWeave becomes the primary component handling HTTP requests/responses
- Axum is either eliminated or reduced to low-level protocol handling
- All HTTP functionality is implemented as StreamWeave transformers
- 100% test coverage for all components
- Performance comparable to or better than Axum
- Full streaming support throughout the pipeline
- Easy composability of middleware and handlers

## Notes

- Each component should be implemented as a separate module in `src/transformers/http/` or `src/producers/http/` or `src/consumers/http/`
- All components must implement the appropriate StreamWeave traits
- Inline tests should be comprehensive and cover edge cases
- Documentation should include usage examples for each component
- Performance benchmarks should be included for critical components
