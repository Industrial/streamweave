# Minimal Syntax Graph Macro Implementation

## Overview

This document describes the implementation of a minimal syntax declarative macro (`graph!`) that supports:
- **Fan-out patterns**: One source to multiple targets
- **Fan-in patterns**: Multiple sources to one target
- **Graph input connections**: External inputs to graph nodes
- **Graph output connections**: Graph nodes to external outputs

## Syntax Design

### Important Notes

- **Node Configuration**: Node configuration (like filter predicates, map transforms, range parameters) is supplied at **runtime via input ports**, not during node creation. Nodes are created with just `Node::new("name")` - no builder methods like `.config()`, `.start()`, `.end()`, etc.
- **Reserved Name**: Nodes cannot be named `graph` - this name is reserved for the graph I/O namespace.

### Basic Syntax

```rust
let graph = graph! {
    // Node definitions (name: NodeInstance)
    // Configuration is supplied at runtime via input ports, not during node creation
    range: RangeNode::new("range"),
    filter: FilterNode::new("filter"),
    square: MapNode::new("square"),
    sum: SumNode::new("sum"),
    
    // All connections must specify explicit port names
    range.out => filter.in,
    filter.out => square.in,
    square.out => sum.in,
    
    // Graph inputs (external -> internal) - use graph. prefix
    // Configuration values are supplied at runtime via graph inputs
    graph.start: 1i32 => range.start,
    graph.end: 11i32 => range.end,
    graph.filter_config => filter.configuration,
    graph.map_config => square.configuration,
    
    // Graph outputs (internal -> external) - use graph. prefix
    sum.out => graph.result
};
```

**Important**: 
- All connections must explicitly specify port names. There are no default ports or sequential connection shortcuts.
- Node configuration is supplied at runtime via input ports (e.g., `graph.filter_config => filter.configuration`), not during node creation.

### Fan-Out Pattern

```rust
let graph = graph! {
    source: SourceNode::new("source"),
    filter1: FilterNode::new("filter1"),
    filter2: FilterNode::new("filter2"),
    filter3: FilterNode::new("filter3"),
    process1: ProcessNode::new("process1"),
    process2: ProcessNode::new("process2"),
    process3: ProcessNode::new("process3"),
    
    // Fan-out: one source port to multiple target ports (all ports must be explicit on both sides)
    source.out => filter1.in,
    source.out => filter2.in,
    source.out => filter3.in,
    
    // Each target continues with explicit ports on both sides
    filter1.out => process1.in,
    filter2.out => process2.in,
    filter3.out => process3.in,
    
    // Configuration supplied at runtime via graph inputs
    graph.filter1_config => filter1.configuration,
    graph.filter2_config => filter2.configuration,
    graph.filter3_config => filter3.configuration
};
```

**Alternative Fan-Out with Different Source Ports:**

```rust
let graph = graph! {
    router: RouterNode::new("router"),
    filter1: FilterNode::new("filter1"),
    filter2: FilterNode::new("filter2"),
    filter3: FilterNode::new("filter3"),
    
    // Fan-out: different source ports to different targets (all ports explicit)
    router.out1 => filter1.in,
    router.out2 => filter2.in,
    router.out3 => filter3.in
};
```

### Fan-In Pattern

```rust
let graph = graph! {
    source1: SourceNode::new("source1"),
    source2: SourceNode::new("source2"),
    source3: SourceNode::new("source3"),
    merge: MergeNode::new("merge"),
    process: ProcessNode::new("process"),
    
    // Fan-in: multiple source ports to one target port (all ports must be explicit on both sides)
    source1.out => merge.in_0,
    source2.out => merge.in_1,
    source3.out => merge.in_2,
    
    // Continue chain after merge with explicit ports on both sides
    merge.out => process.in
};
```

**Alternative Fan-In with Different Source Ports:**

```rust
let graph = graph! {
    source1: SourceNode::new("source1"),
    source2: SourceNode::new("source2"),
    source3: SourceNode::new("source3"),
    merge: MergeNode::new("merge"),
    
    // Fan-in: different source ports to different target ports (all ports explicit)
    source1.out => merge.in_0,
    source2.error => merge.in_1,  // Different source port name
    source3.out => merge.in_2
};
```

### Explicit Port Connections

```rust
let graph = graph! {
    add: AddNode::new("add"),
    
    // Graph inputs with explicit ports - use graph. prefix
    graph.input1 => add.in1,
    graph.input2 => add.in2,
    
    // Graph outputs with explicit ports - use graph. prefix
    add.out => graph.output
};
```

### Combined Patterns

```rust
let graph = graph! {
    source: SourceNode::new("source"),
    filter1: FilterNode::new("filter1"),
    filter2: FilterNode::new("filter2"),
    merge: MergeNode::new("merge"),
    process: ProcessNode::new("process"),
    
    // Fan-out: explicit ports on both source and target sides for each connection
    source.out => filter1.in,
    source.out => filter2.in,
    
    // Fan-in: explicit ports on both source and target sides for each connection
    filter1.out => merge.in_0,
    filter2.out => merge.in_1,
    
    // Continue chain: explicit ports on both sides
    merge.out => process.in,
    
    // Graph I/O: explicit ports with graph. prefix
    graph.external_input => source.in,
    process.out => graph.external_output
};
```

## Macro Implementation Strategy

### Phase 1: Macro Structure

The macro will parse the input and build a `GraphBuilder` structure that can be converted to a `Graph`.

```rust
#[macro_export]
macro_rules! graph {
    // Main entry point - parse the entire graph definition
    ($($tt:tt)*) => {
        $crate::graph::macros::graph_impl! {
            nodes: {},
            edges: [],
            inputs: {},
            outputs: {},
            $($tt)*
        }
    };
}
```

### Phase 2: Parsing Rules

The macro needs to distinguish between:
1. **Node definitions**: `name: NodeInstance` (where `name` cannot be `graph`)
2. **Node-to-node connections**: `source_node.source_port => target_node.target_port` (all ports must be explicit on both sides)
3. **Fan-out**: Multiple connections from one or more source ports to multiple target ports (each connection must explicitly name both source and target ports)
4. **Fan-in**: Multiple connections from multiple source ports to one or more target ports (each connection must explicitly name both source and target ports)
5. **Graph inputs**: `graph.input_name: value => node.port` or `graph.input_name => node.port` (ports must be explicit)
6. **Graph outputs**: `node.port => graph.output_name` (ports must be explicit)

**Critical Rule**: In fan-out and fan-in patterns, **every connection must explicitly name both the source port and the target port**. There are no default port names or shortcuts.

### Phase 3: Builder Generation

The macro generates code that:
1. Creates node instances
2. Adds nodes to the graph
3. Creates edges for connections
4. Exposes input/output ports
5. Connects external channels

## Detailed Implementation

### Core Macro Structure

```rust
// src/graph/macros.rs

use crate::graph::Graph;
use crate::edge::Edge;

/// Internal macro for parsing graph definitions
#[macro_export]
macro_rules! graph_impl {
    // Base case: all tokens consumed
    (
        nodes: {$($nodes:tt)*},
        edges: [$($edges:tt)*],
        inputs: {$($inputs:tt)*},
        outputs: {$($outputs:tt)*},
    ) => {
        $crate::graph::macros::build_graph! {
            nodes: {$($nodes)*},
            edges: [$($edges)*],
            inputs: {$($inputs)*},
            outputs: {$($outputs)*}
        }
    };
    
    // Parse node definition: name: NodeInstance
    (
        nodes: {$($nodes:tt)*},
        edges: [$($edges:tt)*],
        inputs: {$($inputs:tt)*},
        outputs: {$($outputs:tt)*},
        $name:ident : $node:expr,
        $($rest:tt)*
    ) => {
        $crate::graph::macros::graph_impl! {
            nodes: {
                $($nodes)*
                $name: $node,
            },
            edges: [$($edges)*],
            inputs: {$($inputs)*},
            outputs: {$($outputs)*},
            $($rest)*
        }
    };
    
    // Parse node-to-node connection: source_node.source_port => target_node.target_port
    // All ports must be explicitly named - no defaults
    (
        nodes: {$($nodes:tt)*},
        edges: [$($edges:tt)*],
        inputs: {$($inputs:tt)*},
        outputs: {$($outputs:tt)*},
        $source_node:ident . $source_port:ident => $target_node:ident . $target_port:ident,
        $($rest:tt)*
    ) => {
        $crate::graph::macros::graph_impl! {
            nodes: {$($nodes)*},
            edges: [
                $($edges)*
                (stringify!($source_node), stringify!($source_port), stringify!($target_node), stringify!($target_port)),
            ],
            inputs: {$($inputs)*},
            outputs: {$($outputs)*},
            $($rest)*
        }
    };
    
    // Parse graph input with value: graph.input_name: value => node.port
    (
        nodes: {$($nodes:tt)*},
        edges: [$($edges:tt)*],
        inputs: {$($inputs:tt)*},
        outputs: {$($outputs:tt)*},
        graph . $input_name:ident : $value:expr => $node:ident . $port:ident,
        $($rest:tt)*
    ) => {
        $crate::graph::macros::graph_impl! {
            nodes: {$($nodes)*},
            edges: [$($edges)*],
            inputs: {
                $($inputs)*
                $input_name: ($value, stringify!($node), stringify!($port)),
            },
            outputs: {$($outputs)*},
            $($rest)*
        }
    };
    
    // Parse graph input connection: graph.input_name => node.port
    (
        nodes: {$($nodes:tt)*},
        edges: [$($edges:tt)*],
        inputs: {$($inputs:tt)*},
        outputs: {$($outputs:tt)*},
        graph . $input_name:ident => $node:ident . $port:ident,
        $($rest:tt)*
    ) => {
        $crate::graph::macros::graph_impl! {
            nodes: {$($nodes)*},
            edges: [$($edges)*],
            inputs: {
                $($inputs)*
                $input_name: (None, stringify!($node), stringify!($port)),
            },
            outputs: {$($outputs)*},
            $($rest)*
        }
    };
    
    // Parse graph output: node.port => graph.output_name
    (
        nodes: {$($nodes:tt)*},
        edges: [$($edges:tt)*],
        inputs: {$($inputs:tt)*},
        outputs: {$($outputs:tt)*},
        $node:ident . $port:ident => graph . $output_name:ident,
        $($rest:tt)*
    ) => {
        $crate::graph::macros::graph_impl! {
            nodes: {$($nodes)*},
            edges: [$($edges)*],
            inputs: {$($inputs)*},
            outputs: {
                $($outputs)*
                $output_name: (stringify!($node), stringify!($port)),
            },
            $($rest)*
        }
    };
}
```

### Graph Builder Macro

```rust
/// Builds the actual Graph from parsed components
#[macro_export]
macro_rules! build_graph {
    (
        nodes: {
            $($name:ident : $node:expr),+ $(,)?
        },
        edges: [
            $(
                ($source:expr, $source_port:expr, $target:expr, $target_port:expr),
            )+
        ],
        inputs: {
            $(
                $input_name:ident : ($value:expr, $node:expr, $port:expr),
            )*
        },
        outputs: {
            $(
                $output_name:ident : ($node:expr, $port:expr),
            )*
        }
    ) => {{
        let mut graph = $crate::graph::Graph::new("graph".to_string());
        
        // Add all nodes
        $(
            let node_instance = $node;
            graph.add_node(
                stringify!($name).to_string(),
                Box::new(node_instance)
            ).expect("Failed to add node");
        )+
        
        // Add all edges
        $(
            graph.add_edge($crate::edge::Edge {
                source_node: $source.to_string(),
                source_port: $source_port.to_string(),
                target_node: $target.to_string(),
                target_port: $target_port.to_string(),
            }).expect("Failed to add edge");
        )+
        
        // Expose and connect input ports
        $(
            {
                let external_name = stringify!($input_name);
                let internal_node = $node;
                let internal_port = $port;
                
                graph.expose_input_port(
                    internal_node,
                    internal_port,
                    external_name
                ).expect("Failed to expose input port");
                
                // If value is provided, create channel and send value
                if let Some(val) = $value {
                    let (tx, rx) = tokio::sync::mpsc::channel(1);
                    graph.connect_input_channel(external_name, rx)
                        .expect("Failed to connect input channel");
                    
                    // Spawn task to send initial value
                    let val_arc = std::sync::Arc::new(val) as std::sync::Arc<dyn std::any::Any + Send + Sync>;
                    tokio::spawn(async move {
                        let _ = tx.send(val_arc).await;
                    });
                }
            }
        )*
        
        // Expose and connect output ports
        $(
            {
                let external_name = stringify!($output_name);
                let internal_node = $node;
                let internal_port = $port;
                
                graph.expose_output_port(
                    internal_node,
                    internal_port,
                    external_name
                ).expect("Failed to expose output port");
            }
        )*
        
        graph
    }};
}
```

## Simplified Implementation Approach

Given the complexity of macro parsing, here's a more practical approach using a builder pattern:

### Step 1: Create GraphBuilder Helper

```rust
// src/graph/builder.rs

pub struct GraphBuilder {
    name: String,
    nodes: Vec<(String, Box<dyn Node>)>,
    edges: Vec<Edge>,
    input_bindings: Vec<(String, String, String, Option<Arc<dyn Any + Send + Sync>>)>, // (external, node, port, value)
    output_bindings: Vec<(String, String, String)>, // (external, node, port)
}

impl GraphBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            input_bindings: Vec::new(),
            output_bindings: Vec::new(),
        }
    }
    
    pub fn add_node(mut self, name: impl Into<String>, node: Box<dyn Node>) -> Self {
        let name_str = name.into();
        if name_str == "graph" {
            panic!("Node name 'graph' is reserved for graph I/O namespace. Use 'graph.input_name' for graph inputs and 'graph.output_name' for graph outputs.");
        }
        self.nodes.push((name_str, node));
        self
    }
    
    pub fn connect(mut self, source: &str, source_port: &str, target: &str, target_port: &str) -> Self {
        self.edges.push(Edge {
            source_node: source.to_string(),
            source_port: source_port.to_string(),
            target_node: target.to_string(),
            target_port: target_port.to_string(),
        });
        self
    }
    
    // Note: Sequential connections and fan-out/fan-in helpers removed
    // All connections must be explicit with named ports
    
    pub fn input<T: Send + Sync + 'static>(
        mut self,
        external_name: impl Into<String>,
        node: &str,
        port: &str,
        value: Option<T>,
    ) -> Self {
        let value_arc = value.map(|v| Arc::new(v) as Arc<dyn Any + Send + Sync>);
        self.input_bindings.push((
            external_name.into(),
            node.to_string(),
            port.to_string(),
            value_arc,
        ));
        self
    }
    
    pub fn output(mut self, external_name: impl Into<String>, node: &str, port: &str) -> Self {
        self.output_bindings.push((
            external_name.into(),
            node.to_string(),
            port.to_string(),
        ));
        self
    }
    
    pub fn build(mut self) -> Result<Graph, GraphExecutionError> {
        let mut graph = Graph::new(self.name);
        
        // Add nodes
        for (name, node) in self.nodes {
            graph.add_node(name.clone(), node)?;
        }
        
        // Add edges
        for edge in self.edges {
            graph.add_edge(edge)?;
        }
        
        // Setup input bindings
        for (external, node, port, value) in self.input_bindings {
            graph.expose_input_port(&node, &port, &external)?;
            
            if let Some(val) = value {
                let (tx, rx) = tokio::sync::mpsc::channel(1);
                graph.connect_input_channel(&external, rx)?;
                tokio::spawn(async move {
                    let _ = tx.send(val).await;
                });
            }
        }
        
        // Setup output bindings
        for (external, node, port) in self.output_bindings {
            graph.expose_output_port(&node, &port, &external)?;
        }
        
        Ok(graph)
    }
}
```

### Step 2: Simplified Macro

```rust
#[macro_export]
macro_rules! graph {
    (
        $(
            $name:ident : $node:expr
        ),+ $(,)?
        $(
            ; $($connections:tt)*
        )?
    ) => {{
        // Validate that no node is named "graph"
        $(
            $crate::graph::macros::validate_node_name!(stringify!($name));
        )+
        
        let mut builder = $crate::graph::builder::GraphBuilder::new("graph");
        
        // Add nodes
        $(
            builder = builder.add_node(stringify!($name), Box::new($node));
        )+
        
        // Parse connections
        $(
            $crate::graph::macros::parse_connections!(builder, $($connections)*)
        )?
        
        builder.build().expect("Failed to build graph")
    }};
}

/// Validates that a node name is not "graph"
#[macro_export]
macro_rules! validate_node_name {
    ("graph") => {
        compile_error!("Node name 'graph' is reserved for graph I/O namespace. Use 'graph.input_name' for graph inputs and 'graph.output_name' for graph outputs.");
    };
    ($name:expr) => {};
}

#[macro_export]
macro_rules! parse_connections {
    // Parse node-to-node connection: source_node.source_port => target_node.target_port
    // All ports must be explicitly named
    ($builder:ident, $source_node:ident . $source_port:ident => $target_node:ident . $target_port:ident $(, $rest:tt)*) => {
        $builder = $builder.connect(stringify!($source_node), stringify!($source_port), stringify!($target_node), stringify!($target_port));
        $crate::graph::macros::parse_connections!($builder, $($rest)*);
    };
    
    // Parse graph input with value: graph.input_name: value => node.port
    ($builder:ident, graph . $input_name:ident : $value:expr => $node:ident . $port:ident $(, $rest:tt)*) => {
        $builder = $builder.input(stringify!($input_name), stringify!($node), stringify!($port), Some($value));
        $crate::graph::macros::parse_connections!($builder, $($rest)*);
    };
    
    // Parse graph input connection: graph.input_name => node.port
    ($builder:ident, graph . $input_name:ident => $node:ident . $port:ident $(, $rest:tt)*) => {
        $builder = $builder.input(stringify!($input_name), stringify!($node), stringify!($port), None::<()>);
        $crate::graph::macros::parse_connections!($builder, $($rest)*);
    };
    
    // Parse graph output: node.port => graph.output_name
    ($builder:ident, $node:ident . $port:ident => graph . $output_name:ident $(, $rest:tt)*) => {
        $builder = $builder.output(stringify!($output_name), stringify!($node), stringify!($port));
        $crate::graph::macros::parse_connections!($builder, $($rest)*);
    };
    
    ($builder:ident, $(,)?) => {
        $builder
    };
}
```

## Usage Examples

### Example 1: Simple Linear Pipeline

```rust
let graph = graph! {
    range: RangeNode::new("range"),
    filter: FilterNode::new("filter"),
    square: MapNode::new("square"),
    sum: SumNode::new("sum"),
    
    ; range.out => filter.in,
      filter.out => square.in,
      square.out => sum.in,
      graph.start: 1i32 => range.start,
      graph.end: 11i32 => range.end,
      graph.filter_config => filter.configuration,
      graph.map_config => square.configuration,
      sum.out => graph.result
};
```

### Example 2: Fan-Out Pattern

```rust
let graph = graph! {
    source: SourceNode::new("source"),
    filter1: FilterNode::new("filter1"),
    filter2: FilterNode::new("filter2"),
    filter3: FilterNode::new("filter3"),
    
    // Fan-out: each connection must specify explicit ports on BOTH source and target sides
    source.out => filter1.in,
    source.out => filter2.in,
    source.out => filter3.in,
    
    // Configuration supplied at runtime via graph inputs
    graph.filter1_config => filter1.configuration,
    graph.filter2_config => filter2.configuration,
    graph.filter3_config => filter3.configuration
};
```

**Example 2b: Fan-Out with Different Source Ports**

```rust
let graph = graph! {
    router: RouterNode::new("router"),
    filter1: FilterNode::new("filter1"),
    filter2: FilterNode::new("filter2"),
    filter3: FilterNode::new("filter3"),
    
    // Fan-out: different source ports, all explicitly named
    router.out1 => filter1.in,
    router.out2 => filter2.in,
    router.out3 => filter3.in
};
```

### Example 3: Fan-In Pattern

```rust
let graph = graph! {
    source1: SourceNode::new("source1"),
    source2: SourceNode::new("source2"),
    source3: SourceNode::new("source3"),
    merge: MergeNode::new("merge"),
    
    // Fan-in: each connection must specify explicit ports on BOTH source and target sides
    source1.out => merge.in_0,
    source2.out => merge.in_1,
    source3.out => merge.in_2
};
```

**Example 3b: Fan-In with Different Source Ports**

```rust
let graph = graph! {
    source1: SourceNode::new("source1"),
    source2: SourceNode::new("source2"),
    source3: SourceNode::new("source3"),
    merge: MergeNode::new("merge"),
    
    // Fan-in: different source ports, all explicitly named on both sides
    source1.out => merge.in_0,
    source2.error => merge.in_1,  // Different source port name
    source3.out => merge.in_2
};
```

### Example 4: Combined Patterns

```rust
let graph = graph! {
    source: SourceNode::new("source"),
    filter1: FilterNode::new("filter1"),
    filter2: FilterNode::new("filter2"),
    merge: MergeNode::new("merge"),
    process: ProcessNode::new("process"),
    
    // Fan-out: explicit ports for each connection
    source.out => filter1.in,
    source.out => filter2.in,
    
    // Fan-in: explicit ports for each connection
    filter1.out => merge.in_0,
    filter2.out => merge.in_1,
    
    // Continue chain: explicit ports
    merge.out => process.in,
    
    // Graph I/O: explicit ports
    graph.external_input => source.in,
    process.out => graph.external_output
};
```

### Example 5: Invalid - Node Named "graph"

```rust
// This will cause a compile-time error:
let graph = graph! {
    graph: SourceNode::new("graph"),  // ERROR: "graph" is reserved
    // ...
};
```

**Error Message**: `Node name 'graph' is reserved for graph I/O namespace. Use 'graph.input_name' for graph inputs and 'graph.output_name' for graph outputs.`

## Implementation Steps

1. **Create GraphBuilder** (`src/graph/builder.rs`)
   - Implement builder pattern for graph construction
   - Support explicit port connections (no defaults or shortcuts)
   - Handle input/output bindings

2. **Create Macro** (`src/graph/macros.rs`)
   - Implement `graph!` macro
   - Parse node definitions
   - Parse connection patterns
   - Generate builder calls

3. **Add Tests** (`src/graph/macros_test.rs`)
   - Test simple linear pipelines
   - Test fan-out patterns
   - Test fan-in patterns
   - Test graph I/O
   - Test combined patterns

4. **Update Documentation**
   - Add macro usage examples
   - Document syntax patterns
   - Provide migration guide

## Benefits

- **Minimal Syntax**: Reduces boilerplate by 80-90%
- **Visual Clarity**: Connections are immediately visible
- **Type Safety**: Compile-time validation
- **Flexibility**: Supports all graph patterns
- **Explicit Ports**: All connections require explicit port names, preventing ambiguity
- **Clear Namespace**: `graph.` prefix clearly distinguishes graph I/O from node I/O
- **Compile-Time Safety**: Reserved `graph` node name prevents namespace conflicts

## Restrictions

- **Reserved Node Name**: Nodes cannot be named `graph` - this name is reserved for the graph I/O namespace (`graph.input_name` and `graph.output_name`)
- **Required Prefix**: Graph inputs and outputs must use the `graph.` prefix to distinguish them from node-to-node connections
- **Explicit Ports Required**: All connections must explicitly specify port names on **both the source and target sides** (e.g., `node1.out => node2.in`). This applies to:
  - Regular node-to-node connections
  - Fan-out patterns (each connection must name both source and target ports)
  - Fan-in patterns (each connection must name both source and target ports)
  - Graph I/O connections
  - There are no default ports or sequential connection shortcuts
- **Compile-Time Validation**: Attempting to use `graph` as a node name will result in a compile-time error with a helpful message
