# StreamWeave + Axum HTTP Middleware Example

This example demonstrates the **recommended approach** for building HTTP servers with StreamWeave: using Axum as the HTTP server and StreamWeave for business logic and data processing.

## Architecture

### ✅ **Axum + StreamWeave (Recommended)**
- **Axum**: HTTP protocol handling, routing, middleware, connection management
- **StreamWeave**: Business logic pipeline, data transformations, stream processing
- **Clean separation of concerns**
- **Production-ready and maintainable**

### ❌ **Custom HttpServerProducer (Not Recommended)**
- Custom HTTP protocol implementation
- Complex connection management
- More bugs and maintenance overhead
- Reinventing the wheel

## Why This Approach?

### Problems with Custom HttpServerProducer
1. **HTTP Protocol Complexity**: HTTP/1.1 has many edge cases, connection handling, and protocol details
2. **Connection Management**: Proper connection pooling, keep-alive, and cleanup
3. **Error Handling**: HTTP status codes, error responses, and edge cases
4. **Maintenance**: Custom code requires ongoing maintenance and testing
5. **Performance**: Battle-tested HTTP servers are highly optimized

### Benefits of Axum + StreamWeave
1. **Axum is Battle-Tested**: Used in production by many companies
2. **Focus on Business Logic**: StreamWeave handles data processing, not HTTP protocol
3. **Ecosystem**: Rich middleware ecosystem for Axum
4. **Performance**: Highly optimized HTTP server
5. **Maintainability**: Less custom code to maintain

## Example Features

- **HTTP Server**: Axum handles all HTTP protocol details
- **Routing**: Clean route definitions with path parameters
- **Middleware**: CORS, logging, and other HTTP middleware
- **JSON API**: Structured JSON responses with consistent format
- **Error Handling**: Proper HTTP status codes and error responses
- **StreamWeave Integration**: Ready for business logic pipeline integration

## Usage

```bash
# Run the example
cargo run --example 03_axum_middleware

# Test endpoints
curl http://localhost:3004/
curl http://localhost:3004/health
curl http://localhost:3004/version
curl http://localhost:3004/metrics
curl http://localhost:3004/api/users
curl http://localhost:3004/api/users/123
curl -X POST http://localhost:3004/api/users -H "Content-Type: application/json" -d '{"id": 999, "name": "Test", "email": "test@example.com"}'
```

## Integration with StreamWeave

The example includes a `process_request_with_streamweave` function that demonstrates how to integrate StreamWeave middleware pipeline:

```rust
async fn process_request_with_streamweave(
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Vec<u8>,
) -> Result<String, String> {
    // StreamWeave middleware pipeline
    // 1. CORS processing
    // 2. Request logging
    // 3. Request validation
    // 4. Rate limiting
    // 5. Routing
    // 6. Response transformation
}
```

## Key Takeaways

1. **Use Axum for HTTP server functionality**
2. **Use StreamWeave for business logic and data processing**
3. **Avoid custom HTTP protocol implementation**
4. **Focus on what each framework does best**
5. **Leverage the ecosystem and community**

This approach provides the best balance of functionality, performance, and maintainability for production HTTP servers.
