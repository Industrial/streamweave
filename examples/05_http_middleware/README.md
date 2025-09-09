# HTTP Middleware Example

This example demonstrates all the HTTP middleware transformers working together in a comprehensive pipeline.

## Features Demonstrated

### 🔐 Authentication
- **API Key Authentication**: Validates requests using `x-api-key` header
- **Bearer Token Authentication**: Validates JWT tokens in `Authorization` header
- **Role-based Access Control**: Ensures users have required roles
- **Multiple Strategies**: Supports multiple authentication methods

### 🌐 CORS (Cross-Origin Resource Sharing)
- **Origin Validation**: Checks allowed origins
- **Method Validation**: Validates allowed HTTP methods
- **Header Validation**: Validates allowed request headers
- **Preflight Handling**: Handles OPTIONS requests for CORS preflight
- **Credential Support**: Supports credentials in cross-origin requests

### 📝 Logging
- **Request Logging**: Logs incoming requests with metadata
- **Response Logging**: Logs outgoing responses with timing
- **Structured Logging**: JSON and console logging formats
- **Configurable Levels**: Trace, Debug, Info, Warn, Error levels

### 🗜️ Compression
- **Multiple Algorithms**: Gzip, Deflate, and Brotli compression
- **Automatic Detection**: Detects client compression support
- **Size Thresholds**: Only compresses responses above minimum size
- **Quality Control**: Configurable compression quality

### ⏱️ Rate Limiting
- **Token Bucket**: Implements token bucket rate limiting
- **Per-Key Limiting**: Rate limits per API key or IP address
- **Configurable Limits**: Customizable request rates and capacities
- **Graceful Handling**: Returns appropriate rate limit responses

### ✅ Request Validation
- **Body Size Limits**: Enforces maximum request body size
- **Required Headers**: Validates presence of required headers
- **Content Type Validation**: Validates allowed content types
- **Query Parameter Limits**: Limits number of query parameters
- **Header Count Limits**: Limits number of request headers

### 🔄 Response Transformation
- **Security Headers**: Adds security-related headers
- **Cache Headers**: Adds caching directives
- **JSON Minification**: Minifies JSON responses
- **CORS Headers**: Adds CORS headers to responses
- **Custom Headers**: Adds custom response headers

## Running the Example

```bash
# From the project root
cargo run --example http_middleware_example

# Or from the example directory
cd examples/05_http_middleware
cargo run
```

## Test Scenarios

The example processes 8 different test requests:

1. **Authenticated API Request**: Valid API key, gets user list
2. **Bearer Token Request**: Valid JWT token, gets specific user
3. **Large JSON Request**: Tests compression with large payload
4. **CORS Preflight**: OPTIONS request for CORS validation
5. **Public Health Check**: No authentication required
6. **Invalid API Key**: Should be filtered out by auth
7. **Missing Headers**: Should be filtered out by validation
8. **Rate Limit Test**: Tests rate limiting functionality

## Pipeline Flow

```
Request → Auth → CORS → Logging → Validation → Rate Limit → Compression → Response Transform → Response
```

Each middleware transformer:
- **Filters** requests that don't meet criteria
- **Transforms** requests as they pass through
- **Logs** relevant information
- **Adds** security and performance optimizations

## Configuration

The example shows how to configure each middleware transformer:

- **Authentication**: Multiple strategies with role requirements
- **CORS**: Specific origins, methods, and headers
- **Logging**: Log levels and body inclusion
- **Compression**: Algorithms, quality, and size thresholds
- **Rate Limiting**: Token bucket parameters and key extraction
- **Validation**: Size limits, required headers, and content types
- **Response Transform**: Security headers, caching, and minification

## Error Handling

The pipeline gracefully handles:
- **Authentication Failures**: Requests are filtered out
- **Validation Errors**: Invalid requests are rejected
- **Rate Limit Exceeded**: Appropriate error responses
- **CORS Violations**: Proper CORS error responses
- **Compression Errors**: Falls back to uncompressed responses

This example provides a complete foundation for building production-ready HTTP APIs with comprehensive middleware support.