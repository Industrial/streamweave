# HTTP Server Refactoring Plan

## Goal

Refactor the HTTP server in `src/graph/http_server` to be completely definable in terms of a StreamWeave Graph. Users should be able to add nodes, press start, and have an HTTP server running.

## Current Architecture Issues

### Current Problems

1. **HttpGraphServer Wrapper Complexity**
   - Manages channels for request injection
   - Manages GraphExecutor internally  
   - Handles request/response correlation via `HttpResponseCorrelationConsumer`
   - Requires users to understand channels and correlation mechanics

2. **LongLivedHttpRequestProducer**
   - Requires channel receiver to be passed in during construction
   - Users must manually manage channels
   - Breaks the "just build a graph" paradigm

3. **HttpResponseCorrelationConsumer**
   - Requires manual registration of requests
   - Complex correlation logic hidden from users
   - Makes graph construction non-declarative

4. **handler.rs Module**
   - Provides pipeline-based handlers (conflicting API)
   - Creates confusion between graph-based and pipeline-based approaches
   - Not needed if graph-based approach is the primary API

### Current Files Structure

```
src/graph/http_server/
├── mod.rs                    # Module exports
├── graph_server.rs           # HttpGraphServer wrapper (TO REMOVE)
├── handler.rs                # Pipeline handlers (TO REMOVE)
├── middleware.rs             # Axum middleware utilities (KEEP)
├── error.rs                  # Error handling (KEEP)
├── types.rs                  # HTTP types (KEEP)
└── nodes/
    ├── mod.rs                # Node exports
    ├── producer.rs           # HttpRequestProducer, LongLivedHttpRequestProducer (REFACTOR)
    └── consumer.rs           # HttpResponseConsumer, HttpResponseCorrelationConsumer (REFACTOR)
```

## New Architecture Design

### Vision

```rust
// User builds a graph
let graph = GraphBuilder::new()
    .node(HttpServerProducerNode::new(
        "http_server".to_string(),
        HttpServerProducerConfig::default(),
    )?)
    .node(TransformerNode::from_transformer(
        "process".to_string(),
        MyTransformer::new(),
    )?)
    .node(HttpServerConsumerNode::new(
        "http_response".to_string(),
        HttpServerConsumerConfig::default(),
    )?)
    .connect_by_name("http_server", "process")?
    .connect_by_name("process", "http_response")?
    .build();

// User serves the graph
GraphServer::from_graph(graph)
    .serve("0.0.0.0:3000".parse().unwrap())
    .await?;
```

### Key Components

1. **HttpServerProducerNode** (Dedicated Graph Node)
   - Specific graph node type for HTTP server producer (not a wrapper)
   - Accepts Axum requests through internal channel
   - Converts requests to `Message<HttpServerRequest>`
   - No channel receiver needed in constructor
   - Channel managed internally by GraphServer
   - Implements `NodeTrait` directly

2. **HttpServerConsumerNode** (Dedicated Graph Node)
   - Specific graph node type for HTTP server consumer (not a wrapper)
   - Sends responses back to Axum through internal channel
   - Converts `Message<HttpServerResponse>` to Axum responses
   - No channel sender needed in constructor
   - Channel managed internally by GraphServer
   - Implements `NodeTrait` directly

3. **HttpPathRouterNode** (Dedicated Graph Node)
   - Specific graph node type for HTTP path-based routing
   - Routes HTTP requests based on path patterns
   - Works with `Message<HttpServerRequest>`
   - Implements `NodeTrait` directly
   - Replaces the need to wrap `PathBasedRouterTransformer` in `TransformerNode`

4. **GraphServer Integration**
   - Wraps a Graph
   - Creates channels between Axum and graph nodes
   - Starts graph executor
   - Provides Axum Router integration
   - Manages request/response correlation automatically

## Detailed Refactoring Plan

### Phase 1: Create New Producer Node

**File**: `src/graph/http_server/nodes/producer.rs`

**Changes**:
- **Keep**: `HttpRequestProducer` (used by pipeline API, keep for backward compatibility)
- **Remove**: `LongLivedHttpRequestProducer` (replaced by new node)
- **Add**: `HttpServerProducer`
  - Internal channel receiver: `Arc<tokio::sync::Mutex<Option<tokio::sync::mpsc::Receiver<axum::extract::Request>>>>`
  - Method: `set_request_receiver(receiver)` - called by GraphServer
  - Produces: `Message<HttpServerRequest>`
  - Converts Axum requests to HttpServerRequest
  - Handles request extraction (body, headers, etc.)

**API**:
```rust
pub struct HttpServerProducer {
    config: ProducerConfig<Message<HttpServerRequest>>,
    http_config: HttpRequestProducerConfig,
    request_receiver: Arc<tokio::sync::Mutex<Option<tokio::sync::mpsc::Receiver<Request>>>>,
}

impl HttpServerProducer {
    pub fn new(http_config: HttpRequestProducerConfig) -> Self;
    pub fn with_default_config() -> Self;
    pub(crate) fn set_request_receiver(&self, receiver: tokio::sync::mpsc::Receiver<Request>);
}
```

### Phase 2: Create New Consumer Node

**File**: `src/graph/http_server/nodes/consumer.rs`

**Changes**:
- **Keep**: `HttpResponseConsumer` (used by pipeline API, keep for backward compatibility)
- **Remove**: `HttpResponseCorrelationConsumer` (replaced by new node)
- **Add**: `HttpServerConsumer`
  - Internal response sender map: `Arc<tokio::sync::RwLock<HashMap<String, tokio::sync::mpsc::Sender<Response>>>>`
  - Method: `register_request(request_id, sender)` - called by GraphServer
  - Consumes: `Message<HttpServerResponse>`
  - Extracts request_id from message
  - Sends response to corresponding sender
  - Handles timeouts and cleanup

**API**:
```rust
pub struct HttpServerConsumer {
    config: ConsumerConfig<Message<HttpServerResponse>>,
    response_senders: Arc<tokio::sync::RwLock<HashMap<String, tokio::sync::mpsc::Sender<Response>>>>,
    timeout: Duration,
}

impl HttpServerConsumer {
    pub fn new() -> Self;
    pub fn with_timeout(timeout: Duration) -> Self;
    pub(crate) fn register_request(&self, request_id: String, sender: tokio::sync::mpsc::Sender<Response>);
    pub(crate) fn unregister_request(&self, request_id: &str);
}
```

### Phase 4: Create GraphServer Integration

**New File**: `src/graph/http_server/server.rs`

**Purpose**: Integrate Graph with Axum Router

**Implementation Notes**:
- Finds `HttpServerProducerNode` and `HttpServerConsumerNode` in graph
- Creates channels and connects to nodes via `set_request_receiver()` and `register_request()`
- Starts graph executor
- Provides Axum handler and serve methods

**API**:
```rust
pub struct GraphServer {
    graph: Graph,
    executor: Arc<tokio::sync::RwLock<GraphExecutor>>,
    request_sender: tokio::sync::mpsc::Sender<Request>,
    consumer: Arc<HttpServerConsumer>,
    config: GraphServerConfig,
}

pub struct GraphServerConfig {
    pub request_timeout: Duration,
    pub channel_buffer: usize,
    pub http_config: HttpRequestProducerConfig,
}

impl GraphServer {
    pub fn from_graph(graph: Graph) -> Result<Self, GraphServerError>;
    pub fn with_config(graph: Graph, config: GraphServerConfig) -> Result<Self, GraphServerError>;
    pub async fn start(&self) -> Result<(), ExecutionError>;
    pub async fn stop(&self) -> Result<(), ExecutionError>;
    pub fn handler(&self) -> impl Fn(Request) -> Pin<Box<dyn Future<Output = Response<Body>> + Send>> + Send + Sync + Clone + 'static;
    pub async fn serve(self, addr: SocketAddr) -> Result<(), GraphServerError>;
    pub fn into_router(self) -> axum::Router;
}
```

**Implementation Notes**:
- In `from_graph()`: Find HttpServerProducer and HttpServerConsumer nodes in graph
- Create channels and connect to nodes
- Start graph executor
- `handler()`: Creates Axum handler that sends requests to producer and waits for responses
- `serve()`: Creates Axum Router with handler and starts server
- `into_router()`: Returns Axum Router for custom server setup

### Phase 5: Remove Deprecated Code

**Files to Remove**:
1. `src/graph/http_server/graph_server.rs` - Replaced by `server.rs`
2. `src/graph/http_server/handler.rs` - No longer needed (users build graphs directly)

**Files to Update**:
1. `src/graph/http_server/nodes/producer.rs`
   - Remove `LongLivedHttpRequestProducer`
   - Keep `HttpRequestProducer` (for pipeline API)
   - No need to add HttpServerProducer here (it's a node, not a producer trait)

2. `src/graph/http_server/nodes/consumer.rs`
   - Remove `HttpResponseCorrelationConsumer`
   - Keep `HttpResponseConsumer` (for pipeline API)
   - No need to add HttpServerConsumer here (it's a node, not a consumer trait)

3. `src/graph/http_server/nodes/mod.rs`
   - Add exports for new node types:
     - `HttpServerProducerNode`
     - `HttpServerConsumerNode`
     - `HttpPathRouterNode`
   - Update documentation

4. `src/graph/http_server/mod.rs`
   - Update exports
   - Remove `graph_server` and `handler` modules
   - Add `server` module
   - Export new node types from nodes module

### Phase 6: Update Documentation

**Files to Update**:
1. Module-level documentation in `src/graph/http_server/mod.rs`
2. README examples (if they exist)
3. Any architecture documentation

## File Change Summary

### Files to Remove
- `src/graph/http_server/graph_server.rs`
- `src/graph/http_server/handler.rs`

### Files to Add
- `src/graph/http_server/server.rs`
- `src/graph/http_server/nodes/producer_node.rs` (HttpServerProducerNode)
- `src/graph/http_server/nodes/consumer_node.rs` (HttpServerConsumerNode)
- `src/graph/http_server/nodes/path_router_node.rs` (HttpPathRouterNode)

### Files to Update
- `src/graph/http_server/mod.rs`
  - Remove `graph_server` and `handler` modules
  - Add `server` module
  - Update exports
- `src/graph/http_server/nodes/producer.rs`
  - Remove `LongLivedHttpRequestProducer`
  - Keep `HttpRequestProducer` (for pipeline API, backward compatibility)
- `src/graph/http_server/nodes/consumer.rs`
  - Remove `HttpResponseCorrelationConsumer`
  - Keep `HttpResponseConsumer` (for pipeline API, backward compatibility)
- `src/graph/http_server/nodes/mod.rs`
  - Add exports for new node types
  - Update documentation

### Files to Keep (No Changes)
- `src/graph/http_server/error.rs`
- `src/graph/http_server/types.rs`
- `src/graph/http_server/middleware.rs`
- `src/graph/http_server/nodes/path_router_transformer.rs`

## Implementation Steps

1. **Create HttpServerProducerNode**
   - Implement NodeTrait
   - Add request receiver management
   - Convert Axum requests to Message<HttpServerRequest>
   - Handle graph execution (produce stream)

2. **Create HttpServerConsumerNode**
   - Implement NodeTrait
   - Add response sender management
   - Extract request_id from messages
   - Send responses to correct channels
   - Handle graph execution (consume stream)

3. **Create HttpPathRouterNode**
   - Implement NodeTrait
   - Add routing logic (based on PathBasedRouterTransformer)
   - Handle multiple output ports
   - Preserve message IDs through routing

4. **Create GraphServer integration**
   - Implement node discovery (find HttpServerProducerNode and HttpServerConsumerNode in graph)
   - Create channels and connect to nodes via set_request_receiver() and register_request()
   - Implement Axum handler
   - Implement serve() and into_router() methods

5. **Update module exports**
   - Remove deprecated modules
   - Add new server module
   - Export new node types
   - Update public API

6. **Remove deprecated code**
   - Remove HttpGraphServer
   - Remove LongLivedHttpRequestProducer
   - Remove HttpResponseCorrelationConsumer
   - Remove handler.rs

7. **Update tests** (if they exist)
   - Update tests to use new API
   - Add tests for new node types
   - Add tests for GraphServer

8. **Update documentation**
   - Update module docs
   - Update examples to show node usage
   - Update architecture docs

## Backward Compatibility

**Breaking Changes**:
- `HttpGraphServer` removed (users must migrate to `GraphServer`)
- `LongLivedHttpRequestProducer` removed (replaced by `HttpServerProducerNode`)
- `HttpResponseCorrelationConsumer` removed (replaced by `HttpServerConsumerNode`)
- `handler.rs` functions removed (users should use graph-based approach)
- Users must use dedicated node types instead of wrapping in ProducerNode/ConsumerNode/TransformerNode

**Non-Breaking Changes**:
- `HttpRequestProducer` kept (used by pipeline API)
- `HttpResponseConsumer` kept (used by pipeline API)
- `PathBasedRouterTransformer` kept (can still be used, but HttpPathRouterNode is preferred for graphs)
- All types, error handling, middleware unchanged

## Migration Guide

### Before (Current API)
```rust
let (tx, rx) = mpsc::channel(100);
let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
        "http_server".to_string(),
        LongLivedHttpRequestProducer::new(rx, config),
    )?)
    .node(ConsumerNode::from_consumer(
        "http_response".to_string(),
        HttpResponseCorrelationConsumer::with_timeout(timeout),
    )?)
    .build();

let (server, request_receiver) = HttpGraphServer::new(graph, config).await?;
server.start().await?;
let handler = server.create_handler();
let app = Router::new().route("/api/*", get(handler));
```

### After (New API)
```rust
let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
        "http_server".to_string(),
        HttpServerProducer::with_default_config(),
    )?)
    .node(ConsumerNode::from_consumer(
        "http_response".to_string(),
        HttpServerConsumer::new(),
    )?)
    .build();

let server = GraphServer::from_graph(graph)?;
server.serve("0.0.0.0:3000".parse().unwrap()).await?;
```

## Testing Strategy

1. **Unit Tests**
   - Test HttpServerProducer request conversion
   - Test HttpServerConsumer response sending
   - Test GraphServer node discovery
   - Test channel management

2. **Integration Tests**
   - Test full HTTP request/response cycle
   - Test multiple concurrent requests
   - Test error handling
   - Test timeout handling

3. **Examples**
   - Simple echo server
   - JSON API server
   - Server with routing
   - Server with middleware

