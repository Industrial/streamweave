# StreamWeave HTTP Server Core Components TODO

# StreamWeave HTTP Server Core Components TODO

> **⚠️ ARCHITECTURAL UPDATE (2024)**: This document has been updated to reflect the current recommended approach of using **Axum + StreamWeave** instead of a pure StreamWeave HTTP server implementation.

## Current Recommended Architecture ✅

**Axum + StreamWeave Integration** (as demonstrated in `examples/03_axum_middleware`):
- **Axum**: Handles HTTP protocol, connection management, routing, and low-level HTTP details
- **StreamWeave**: Handles business logic, data processing, middleware pipeline, and transformations
- **Architecture**: `Axum HTTP Server → StreamWeave Pipeline → Response`
- **Benefits**: Battle-tested HTTP server + StreamWeave's streaming-first data processing

## Why Axum + StreamWeave Instead of Pure StreamWeave?

### Problems with Custom HTTP Server Implementation
1. **HTTP Protocol Complexity**: HTTP/1.1 and HTTP/2 have many edge cases, connection handling, and protocol details
2. **Connection Management**: Proper connection pooling, keep-alive, and cleanup require extensive testing
3. **Error Handling**: HTTP status codes, error responses, and edge cases are complex
4. **Maintenance Overhead**: Custom HTTP server code requires ongoing maintenance and testing
5. **Performance**: Battle-tested HTTP servers like Axum are highly optimized
6. **Ecosystem**: Rich middleware ecosystem and community support

### Benefits of Axum + StreamWeave
1. **Focus on Strengths**: Axum handles HTTP protocol, StreamWeave handles data processing
2. **Production Ready**: Axum is used in production by many companies
3. **Maintainability**: Less custom code to maintain and debug
4. **Performance**: Highly optimized HTTP server with proven track record
5. **Ecosystem**: Access to Axum's rich middleware ecosystem
6. **Clean Separation**: Clear boundaries between HTTP concerns and business logic

## Superseded Components

The following components are **NO LONGER NEEDED** as they have been superseded by Axum:

---

## Historical Context (Superseded)

This document originally outlined the components needed to make StreamWeave the primary architectural component of an HTTP server, replacing the current secondary role where it's only used within Axum route handlers.

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

## 1. HTTP Router Transformer ✅ COMPLETED

**Priority: HIGH** | **Estimated Effort: 3-4 days** | **Status: COMPLETED**

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

## 2. HTTP Middleware System ✅ COMPLETED

**Priority: HIGH** | **Estimated Effort: 2-3 days** | **Status: COMPLETED**

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

## 3. HTTP Response Builder Transformer ✅ COMPLETED

**Priority: HIGH** | **Estimated Effort: 2-3 days** | **Status: ✅ COMPLETED**

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

### Inline Tests Required (100% Coverage) ✅ COMPLETED
- ✅ Response building with different status codes
- ✅ Header management and validation
- ✅ Content type detection and setting
- ✅ Streaming response handling
- ✅ Error response formatting
- ✅ Chunked transfer encoding
- ✅ Compression integration
- ✅ Large response handling
- ✅ Concurrent response building
- ✅ Memory usage optimization

### Implementation Status: ✅ FULLY COMPLETED
- **100% test coverage achieved** with comprehensive unit tests
- **All builder utilities implemented**: JsonResponseBuilder, HtmlResponseBuilder, TextResponseBuilder, ErrorResponseBuilder
- **Convenience functions provided** via responses module
- **Security features integrated**: automatic security headers, CORS support
- **Examples updated**: HTTP Router example now demonstrates HTTP Response Builder integration
- **Full streaming support** for response generation

---

## 4. HTTP Server Producer ❌ SUPERSEDED BY AXUM

**Priority: N/A** | **Estimated Effort: N/A** | **Status: SUPERSEDED**

### Context
~~A producer that accepts incoming HTTP connections and produces HTTP request streams, replacing the need for Axum's server functionality.~~

**SUPERSEDED**: This component is no longer needed as Axum handles HTTP server functionality. Axum provides battle-tested HTTP protocol handling, connection management, and server implementation that would be complex and error-prone to reimplement.

### Why Superseded?
- **HTTP Protocol Complexity**: HTTP/1.1 and HTTP/2 have many edge cases that Axum already handles
- **Connection Management**: Axum provides robust connection pooling and keep-alive handling
- **TLS/SSL Support**: Axum integrates seamlessly with Rust's TLS ecosystem
- **Performance**: Axum is highly optimized and battle-tested in production
- **Maintenance**: Custom HTTP server implementation would require ongoing maintenance

### Current Solution
Use Axum as the HTTP server with StreamWeave handling business logic in route handlers (see `examples/03_axum_middleware`).| **Estimated Effort: 3-4 days**

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

## 5. HTTP Response Consumer ❌ SUPERSEDED BY AXUM

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

## 6. Axum Types Integration ❌ SUPERSEDED BY AXUM TYPES

**Priority: MEDIUM** | **Estimated Effort: 1-2 days**

### Context
~~More sophisticated HTTP types for better integration and easier manipulation within StreamWeave pipelines.~~

**SUPERSEDED**: This component is no longer needed as Axum provides all necessary HTTP types. Custom HTTP types create unnecessary abstraction layers and conversion overhead.

### Why Superseded?
- **Type Duplication**: Axum already provides `Request<B>`, `Response<B>`, `Method`, `Uri`, `HeaderMap`, etc.
- **Performance Overhead**: Converting between Axum and custom types adds unnecessary overhead
- **Ecosystem Integration**: Axum types integrate seamlessly with Tower middleware and extractors
- **Maintenance Burden**: Custom types require ongoing maintenance to stay in sync with Axum
- **Limited Value**: No significant benefits over using Axum types directly

### Current Solution
Use Axum's built-in types directly in StreamWeave pipelines:
- `Request<B>` for incoming requests
- `Response<B>` for outgoing responses  
- `Stream` for streaming bodies
- `Path<T>`, `Query<T>`, `TypedHeader<T>` extractors for parameters
- `ConnectInfo` for connection information

### Migration TODO List

#### REMOVE (No Longer Needed)
- [x] Remove `StreamWeaveHttpRequest` struct
- [x] Remove `StreamWeaveHttpResponse` struct  
- [x] Remove `ConnectionInfo` struct
- [x] Remove custom HTTP type serialization/deserialization
- [x] Remove custom parameter extraction logic
- [x] Remove custom header manipulation utilities
- [x] Remove custom streaming body handling
- [x] Remove custom error type mappings
- [x] Remove HTTP type conversion functions
- [x] Remove custom type tests and benchmarks

#### CHANGE (Update Existing Code)
- [ ] Update `examples/03_axum_middleware/main.rs` to use Axum types directly
- [ ] Remove conversion between Axum and StreamWeave types
- [ ] Update StreamWeave transformers to accept Axum `Request<B>` instead of `StreamWeaveHttpRequestChunk`
- [ ] Update StreamWeave transformers to return Axum `Response<B>` instead of `StreamWeaveHttpResponse`
- [ ] Update pipeline builders to work with Axum types
- [ ] Update middleware transformers to use Axum extractors
- [ ] Update router transformer to use Axum routing patterns
- [ ] Update response transformers to return Axum response types
- [ ] Update error handling to use Axum error types
- [ ] Update logging to work with Axum request/response types

#### ADD (New Axum Integration)
- [ ] Add Axum `Stream` extractor support in StreamWeave pipelines
- [ ] Add Axum `Path<T>` extractor integration for path parameters
- [ ] Add Axum `Query<T>` extractor integration for query parameters
- [ ] Add Axum `TypedHeader<T>` extractor integration for headers
- [ ] Add Axum `ConnectInfo` extractor integration for connection info
- [ ] Add Axum `Json<T>` response integration
- [ ] Add Axum `Html<T>` response integration
- [ ] Add Axum `Text<T>` response integration
- [ ] Add Axum streaming response support
- [ ] Add Axum error response integration
- [ ] Add Axum middleware integration helpers
- [ ] Add Axum extractor trait implementations for StreamWeave
- [ ] Add Axum response trait implementations for StreamWeave
- [ ] Add Axum type conversion utilities (if needed)
- [ ] Add Axum integration examples
- [ ] Add Axum type documentation
- [ ] Add Axum integration tests
- [ ] Add performance benchmarks for Axum vs custom types
- [ ] Add migration guide from custom types to Axum types

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

## Current Implementation Status

### ✅ COMPLETED COMPONENTS
1. **HTTP Router Transformer** - Routes requests within StreamWeave pipelines
2. **HTTP Middleware System** - CORS, logging, validation, rate limiting, compression, response transformation
3. **HTTP Response Builder** - Constructs HTTP responses from processed data

### ❌ SUPERSEDED COMPONENTS (No Longer Needed)
4. **HTTP Server Producer** - Superseded by Axum HTTP server
5. **HTTP Response Consumer** - Superseded by Axum response handling
6. **Connection Management Transformer** - Superseded by Axum connection management
7. **HTTP Error Handling System** - Superseded by Axum error handling
8. **HTTP Pipeline Builder** - Superseded by Axum + StreamWeave integration pattern
9. **HTTP Server Integration Tests** - Superseded by Axum + StreamWeave integration tests

### ❌ SUPERSEDED COMPONENTS (No Longer Needed)
6. **Enhanced HTTP Types** - Superseded by Axum types

## Current Recommended Architecture

**Axum + StreamWeave Integration** (see `examples/03_axum_middleware`):
- **Axum**: HTTP server, connection management, routing, error handling
- **StreamWeave**: Business logic, data processing, middleware pipeline
- **Benefits**: Production-ready HTTP server + streaming-first data processing

## Success Criteria (Updated)

- ✅ StreamWeave integrates seamlessly with Axum for HTTP server functionality
- ✅ Axum handles HTTP protocol details and connection management
- ✅ StreamWeave handles business logic and data processing pipelines
- ✅ Clean separation of concerns between HTTP and business logic
- ✅ Production-ready and maintainable architecture
- ✅ Full streaming support throughout the StreamWeave pipeline
- ✅ Easy composability of middleware and transformers

## Summary

This TODO document has been updated to reflect the architectural decision to use **Axum + StreamWeave** instead of a pure StreamWeave HTTP server implementation. The key insight is that HTTP server implementation is complex and error-prone, while Axum provides battle-tested HTTP functionality that can be seamlessly integrated with StreamWeave's streaming-first data processing capabilities.

### Key Changes Made:
1. **Superseded Components**: Marked HTTP Server Producer, HTTP Response Consumer, Connection Management, Error Handling, and Pipeline Builder as superseded by Axum
2. **Current Architecture**: Documented the recommended Axum + StreamWeave integration pattern
3. **Updated Success Criteria**: Focused on integration rather than replacement of Axum
4. **Example Reference**: Pointed to `examples/03_axum_middleware` as the current best practice

### Next Steps:
- Focus on enhancing StreamWeave's data processing capabilities
- Improve integration patterns with Axum
- Add more sophisticated middleware transformers
- Optimize performance of StreamWeave pipelines
- Expand examples and documentation

## Notes

- StreamWeave components should focus on data processing and business logic
- HTTP protocol concerns should be handled by Axum
- Integration examples should demonstrate clean separation of concerns
- Performance optimization should focus on StreamWeave pipeline efficiency
- Documentation should emphasize the Axum + StreamWeave integration pattern