# Visualization HTTP Server Example

This example demonstrates how to serve pipeline visualizations via HTTP. It creates a web server that serves DAG visualizations and provides API endpoints for DAG data.

## Overview

This example:
1. Creates a multi-stage pipeline
2. Generates a DAG representation
3. Starts an HTTP server
4. Serves visualization UI at the root path
5. Provides API endpoints for DAG data (JSON and DOT formats)
6. Supports graceful shutdown

## Prerequisites

**The `http-server` feature must be enabled** for this example to work.

## Running the Example

```bash
cargo run --example visualization_server --features http-server
```

## Expected Output

```
🎨 StreamWeave Visualization HTTP Server Example
================================================

📊 Generating pipeline DAG...
✅ DAG generated successfully!
   Nodes: 4
   Edges: 3

🌐 Starting HTTP server...
   Server address: http://127.0.0.1:8080
   UI: http://127.0.0.1:8080/
   DAG API: http://127.0.0.1:8080/api/dag
   DAG DOT: http://127.0.0.1:8080/api/dag/dot

💡 Press Ctrl+C to stop the server
```

## Server Endpoints

### Root (`/`)

Serves the interactive HTML visualization UI.

**Response:**
- Content-Type: `text/html; charset=utf-8`
- Body: Standalone HTML with embedded DAG visualization

**Example:**
```bash
curl http://127.0.0.1:8080/
```

### DAG JSON API (`/api/dag`)

Returns the DAG structure as JSON.

**Response:**
- Content-Type: `application/json`
- Body: DAG structure in JSON format

**Example:**
```bash
curl http://127.0.0.1:8080/api/dag
```

**Response Format:**
```json
{
  "nodes": [
    {
      "id": "producer",
      "kind": "Producer",
      "metadata": { ... }
    },
    ...
  ],
  "edges": [
    {
      "source": "producer",
      "target": "transformer1",
      "data_type": "i32"
    },
    ...
  ]
}
```

### DAG DOT API (`/api/dag/dot`)

Returns the DAG structure in DOT format (for Graphviz).

**Response:**
- Content-Type: `text/plain; charset=utf-8`
- Body: DAG structure in DOT format

**Example:**
```bash
curl http://127.0.0.1:8080/api/dag/dot
```

**Response Format:**
```
digraph {
  producer -> transformer1;
  transformer1 -> transformer2;
  transformer2 -> consumer;
  ...
}
```

## Server Configuration

### Default Address

The server binds to `127.0.0.1:8080` by default.

### Changing the Address

To change the server address, modify the `addr` variable in `main.rs`:

```rust
let addr: SocketAddr = "0.0.0.0:3000".parse()?;
```

### Graceful Shutdown

The server supports graceful shutdown via:
- **Ctrl+C** (SIGINT)
- **SIGTERM** (on Unix systems)

When a shutdown signal is received, the server:
1. Stops accepting new connections
2. Completes in-flight requests
3. Closes all connections
4. Exits cleanly

## Usage Examples

### View Visualization in Browser

1. Start the server:
   ```bash
   cargo run --example visualization_server --features http-server
   ```

2. Open browser to:
   ```
   http://127.0.0.1:8080/
   ```

### Get DAG JSON Programmatically

```bash
curl http://127.0.0.1:8080/api/dag | jq
```

### Generate Graphviz Image from DOT

```bash
curl http://127.0.0.1:8080/api/dag/dot | dot -Tpng -o dag.png
```

### Integration with Other Tools

The JSON API can be consumed by other tools:

```python
import requests

response = requests.get('http://127.0.0.1:8080/api/dag')
dag = response.json()

for node in dag['nodes']:
    print(f"Node: {node['id']}, Kind: {node['kind']}")
```

## Code Structure

- `main.rs`: Main entry point with HTTP server setup
- `pipeline.rs`: Pipeline component creation

## Key Components

### Server Setup

The server uses Axum framework:

```rust
let app = Router::new()
    .route("/", get(serve_ui))
    .route("/api/dag", get(serve_dag_json))
    .route("/api/dag/dot", get(serve_dag_dot))
    .with_state(dag_state);
```

### State Management

DAG is stored in shared state:

```rust
let dag_state = Arc::new(RwLock::new(dag));
```

### Route Handlers

- `serve_ui()`: Serves HTML visualization
- `serve_dag_json()`: Serves DAG as JSON
- `serve_dag_dot()`: Serves DAG as DOT format

## Feature Requirements

This example requires the `http-server` feature:

```toml
[features]
http-server = ["dep:axum", "dep:tower", ...]
```

### Enabling the Feature

**Option 1: Command Line**
```bash
cargo run --example visualization_server --features http-server
```

**Option 2: Cargo.toml**
```toml
[features]
default = ["http-server"]
```

**Option 3: Example-specific**
```toml
[[example]]
name = "visualization_server"
required-features = ["http-server"]
```

## Error Handling

If the `http-server` feature is not enabled, the example will display:

```
❌ HTTP server feature is not enabled

📦 To enable this example:
   1. Enable the 'http-server' feature:
      cargo run --example visualization_server --features http-server
```

## Production Considerations

### Security

- **Binding Address**: Use `127.0.0.1` for local-only access, `0.0.0.0` for network access
- **CORS**: Add CORS headers if serving from different origins
- **Authentication**: Add authentication middleware for production use
- **HTTPS**: Use reverse proxy (nginx, Caddy) for HTTPS in production

### Performance

- **Connection Pooling**: Axum handles connection pooling automatically
- **Concurrent Requests**: Axum handles concurrent requests efficiently
- **Caching**: Consider caching DAG generation for frequently accessed endpoints

### Monitoring

- **Health Check**: Add `/health` endpoint for monitoring
- **Metrics**: Integrate with metrics collection (Prometheus, etc.)
- **Logging**: Add structured logging for request tracking

## Related Examples

- `visualization_basic`: Basic console export
- `visualization_html`: Static HTML generation
- `visualization_metrics`: Metrics visualization
- `http_graph_server`: Full HTTP graph server example

## Troubleshooting

### Port Already in Use

If port 8080 is already in use:

```rust
let addr: SocketAddr = "127.0.0.1:8081".parse()?;
```

### Feature Not Available

If you see "HTTP server feature is not enabled":
1. Check that `http-server` feature is enabled
2. Verify dependencies in `Cargo.toml`
3. Rebuild with `cargo clean && cargo build --features http-server`

### Server Won't Start

- Check firewall settings
- Verify address format (must be valid SocketAddr)
- Check for port conflicts
- Review error messages in console

## Advanced Usage

### Custom Routes

Add custom routes:

```rust
let app = Router::new()
    .route("/", get(serve_ui))
    .route("/api/dag", get(serve_dag_json))
    .route("/api/health", get(health_check))
    .with_state(dag_state);
```

### Middleware

Add middleware (CORS, logging, etc.):

```rust
use tower_http::cors::CorsLayer;

let app = Router::new()
    .route("/", get(serve_ui))
    .layer(CorsLayer::permissive())
    .with_state(dag_state);
```

### Dynamic DAG Updates

Update DAG at runtime:

```rust
let mut dag = dag_state.write().await;
// Modify DAG
*dag = new_dag;
```

## Tips

1. **Development**: Use `127.0.0.1` for local development
2. **Testing**: Use different ports for multiple instances
3. **Production**: Use reverse proxy for HTTPS and load balancing
4. **Monitoring**: Add health check endpoint for monitoring systems
5. **Documentation**: Document API endpoints for team use

