# HTTP Graph Server Example

This example demonstrates building a complete HTTP web framework using StreamWeave's Graph API. All HTTP traffic flows through a single graph, enabling powerful patterns like path-based routing, fan-out, fan-in, and complex request processing pipelines.

## Prerequisites

Before running this example, you need:

1. **HTTP server feature enabled** - Build with `--features http-server`
2. **Tokio runtime** - Required for async HTTP server

## Quick Start

```bash
# Run the HTTP Graph Server example
cargo run --example http_graph_server --features http-server
```

The server will start on `http://127.0.0.1:3000` and you can test the endpoints using curl or any HTTP client.

## What This Example Demonstrates

### 1. Long-Lived Graph-Based HTTP Server

The HTTP Graph Server maintains a single long-lived graph executor that processes all HTTP requests. This enables:

- **Persistent State**: The graph maintains state across requests
- **Resource Sharing**: Shared resources (connections, caches, etc.) can be managed in the graph
- **Complex Processing**: Requests can flow through complex transformation pipelines

### 2. Path-Based Routing

The `PathBasedRouterTransformer` routes requests to different handlers based on URL path patterns:

- `/api/rest/*` → REST API handler
- `/api/graphql` → GraphQL-style handler
- `/api/rpc/*` → RPC handler
- `/static/*` → Static file handler
- `/api/fanout` → Fan-out example
- Default → 404 handler

### 3. Multiple Handler Types

The example demonstrates different handler patterns:

- **REST Handler**: Traditional REST API with GET/POST methods
- **GraphQL Handler**: GraphQL-style query processing
- **RPC Handler**: JSON-RPC style method calls
- **Static Handler**: File serving
- **Fan-Out Handler**: One request processed by multiple handlers
- **Default Handler**: 404 and health check endpoints

### 4. Request/Response Correlation

The `HttpResponseCorrelationConsumer` automatically matches responses to their original requests using `request_id`. This enables:

- **Async Processing**: Requests can be processed asynchronously
- **Response Matching**: Responses are automatically routed back to the correct client
- **Timeout Handling**: Requests that don't receive responses within the timeout are handled gracefully

## Testing the Endpoints

Once the server is running, you can test the endpoints:

### REST API

```bash
# List users
curl http://127.0.0.1:3000/api/rest/users

# Create user
curl -X POST http://127.0.0.1:3000/api/rest/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Alice", "email": "alice@example.com"}'

# List posts
curl http://127.0.0.1:3000/api/rest/posts
```

### GraphQL-Style

```bash
curl -X POST http://127.0.0.1:3000/api/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ users { id name email } }"}'
```

### RPC

```bash
# Echo RPC method
curl -X POST http://127.0.0.1:3000/api/rpc/echo \
  -H "Content-Type: application/json" \
  -d '{"params": {"message": "Hello, StreamWeave!"}}'

# Add RPC method
curl -X POST http://127.0.0.1:3000/api/rpc/add \
  -H "Content-Type: application/json" \
  -d '{"params": {"a": 5, "b": 3}}'
```

### Static Files

```bash
# Get index.html
curl http://127.0.0.1:3000/static/index.html

# Get CSS
curl http://127.0.0.1:3000/static/style.css

# Get JavaScript
curl http://127.0.0.1:3000/static/app.js
```

### Fan-Out Example

```bash
curl http://127.0.0.1:3000/api/fanout
```

### Health Check

```bash
curl http://127.0.0.1:3000/api/health
```

## Architecture

The example demonstrates the following architecture:

```
HTTP Request (Axum)
    ↓
LongLivedHttpRequestProducer
    ↓
PathBasedRouterTransformer
    ↓ (routes by path)
    ├─→ REST Handler
    ├─→ GraphQL Handler
    ├─→ RPC Handler
    ├─→ Static Handler
    ├─→ Fan-Out Handler
    └─→ Default Handler
    ↓
HttpResponseCorrelationConsumer
    ↓
HTTP Response (Axum)
```

## Key Concepts

### Graph-Based Processing

All HTTP requests flow through a single graph, enabling:

- **Complex Transformations**: Requests can be transformed by multiple nodes
- **Fan-Out**: One request can be processed by multiple handlers
- **Fan-In**: Multiple sources can be merged into one response
- **Stateful Processing**: Nodes can maintain state across requests

### Path-Based Routing

The `PathBasedRouterTransformer` provides flexible routing:

- **Pattern Matching**: Supports wildcard patterns (`/api/rest/*`)
- **Multiple Output Ports**: Routes to different handlers based on path
- **Default Route**: Handles unmatched requests

### Request/Response Correlation

The `HttpResponseCorrelationConsumer` ensures responses are matched to requests:

- **Request ID**: Each request gets a unique `request_id`
- **Automatic Matching**: Responses are matched using `request_id`
- **Timeout Handling**: Requests that timeout are handled gracefully

## Extending the Example

This example can be extended to demonstrate:

- **Fan-Out Patterns**: One request processed by multiple handlers with results merged
- **Fan-In Patterns**: Multiple sources merged into one response
- **Stateful Nodes**: Nodes that maintain state across requests
- **Middleware**: Request/response transformation nodes
- **Error Handling**: Error handling strategies in the graph
- **Rate Limiting**: Rate limiting transformers
- **Caching**: Caching transformers for responses

## Best Practices

1. **Use Path-Based Routing**: Route requests early in the graph
2. **Correlate Requests/Responses**: Always use `request_id` for correlation
3. **Handle Timeouts**: Set appropriate timeouts for request processing
4. **Error Handling**: Use appropriate error handling strategies
5. **State Management**: Use stateful nodes for shared resources

