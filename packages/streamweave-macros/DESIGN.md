# StreamWeave Macro-Based DSL Design Document

## Executive Summary

This document designs a macro-based Domain-Specific Language (DSL) for StreamWeave that provides concise, expressive syntax for building both linear pipelines and complex graphs while maintaining full type safety and compile-time validation. The DSL will be built using Rust's procedural macro system with `syn`, `proc-macro2`, and `quote`.

## Table of Contents

1. [Goals and Requirements](#goals-and-requirements)
2. [Macro Library Research](#macro-library-research)
3. [Pipeline DSL Design](#pipeline-dsl-design)
4. [Graph DSL Design](#graph-dsl-design)
5. [Implementation Strategy](#implementation-strategy)
6. [Examples and Use Cases](#examples-and-use-cases)
7. [Migration Path](#migration-path)
8. [Future Enhancements](#future-enhancements)

---

## Goals and Requirements

### Primary Goals

1. **Conciseness**: Reduce boilerplate while maintaining clarity
2. **Type Safety**: Preserve compile-time type checking
3. **Expressiveness**: Support both simple and complex topologies
4. **Backward Compatibility**: Work alongside existing builder APIs
5. **Error Messages**: Provide clear, helpful compile-time errors

### Requirements

- Must compile to existing `PipelineBuilder` and `GraphBuilder` APIs
- Must preserve all type safety guarantees
- Must support async operations (transformers)
- Must handle error strategies
- Must support multiple transformers in sequence
- Must support fan-in/fan-out patterns in graphs
- Must provide good IDE support (autocomplete, error messages)

---

## Macro Library Research

### Core Libraries

#### 1. **syn** (`/dtolnay/syn`)
- **Purpose**: Parse Rust code into syntax trees
- **Key Features**:
  - Comprehensive Rust syntax tree representation
  - Custom parsing via `Parse` trait
  - Excellent error reporting with span tracking
  - Support for custom syntax via `ParseStream`
- **Use Case**: Parse DSL syntax into structured AST

#### 2. **proc-macro2** (`/dtolnay/proc-macro2`)
- **Purpose**: Stable API for procedural macros
- **Key Features**:
  - Works outside proc-macro context (testable)
  - Token stream manipulation
  - Stable across Rust versions
- **Use Case**: Foundation for all procedural macros

#### 3. **quote** (Standard Rust macro library)
- **Purpose**: Generate Rust code from templates
- **Key Features**:
  - `quote!` macro for code generation
  - Variable interpolation with `#var`
  - Repetition with `#(...)*`
- **Use Case**: Generate builder code from DSL

#### 4. **derive_builder** (`/colin-kiegel/rust-derive-builder`)
- **Purpose**: Auto-generate builder patterns
- **Key Patterns**:
  - Type-state builders
  - Fluent API generation
  - Validation at build time
- **Use Case**: Reference for builder pattern generation

### Design Principles from Research

1. **Parse, Don't Validate**: Parse syntax first, validate semantics later
2. **Span Preservation**: Keep source locations for error messages
3. **Incremental Parsing**: Use `ParseStream` for complex syntax
4. **Code Generation**: Use `quote!` for type-safe code generation
5. **Testability**: Use `proc-macro2` for unit testing

---

## Pipeline DSL Design

### Syntax Overview

The pipeline DSL provides a concise syntax for linear data flows:

```rust
// Current API (verbose)
let pipeline = PipelineBuilder::new()
    .producer(ArrayProducer::new(vec![1, 2, 3]))
    .transformer(MapTransformer::new(|x| x * 2))
    .consumer(VecConsumer::new());

// DSL (concise)
let pipeline = pipeline! {
    ArrayProducer::new(vec![1, 2, 3])
    => MapTransformer::new(|x| x * 2)
    => VecConsumer::new()
};
```

### Syntax Grammar

```
pipeline := 'pipeline!' '{' pipeline_body '}'
pipeline_body := producer (arrow transformer)* arrow consumer
arrow := '=>' | '->'
producer := expression
transformer := expression
consumer := expression
```

### Detailed Syntax

#### Basic Pipeline

```rust
pipeline! {
    producer_expr => transformer_expr => consumer_expr
}
```

#### Multiple Transformers

```rust
pipeline! {
    producer_expr
    => transformer1
    => transformer2
    => consumer_expr
}
```

#### With Error Strategy

```rust
pipeline! {
    producer_expr
    => transformer_expr
    => consumer_expr
    with ErrorStrategy::Skip
}
```

#### With Configuration

```rust
pipeline! {
    producer_expr
    => transformer_expr
    => consumer_expr
    config {
        error_strategy: ErrorStrategy::Retry(3)
    }
}
```

### AST Structure

```rust
struct PipelineMacro {
    producer: Expr,
    transformers: Vec<Expr>,
    consumer: Expr,
    config: Option<PipelineConfig>,
}

struct PipelineConfig {
    error_strategy: Option<Expr>,
}
```

### Generated Code

The macro expands to:

```rust
// Input:
pipeline! {
    ArrayProducer::new(vec![1, 2, 3])
    => MapTransformer::new(|x| x * 2)
    => VecConsumer::new()
}

// Output:
{
    PipelineBuilder::new()
        .producer(ArrayProducer::new(vec![1, 2, 3]))
        .transformer(MapTransformer::new(|x| x * 2))
        .await
        .consumer(VecConsumer::new())
}
```

### Type Safety

The macro preserves all type safety:
- Producer output type must match transformer input type
- Transformer output type must match next transformer input type
- Final transformer output must match consumer input type
- All checked at compile time via existing builder types

---

## Graph DSL Design

### Syntax Overview

The graph DSL provides a concise syntax for complex topologies:

```rust
// Current API (verbose)
let mut builder = GraphBuilder::new();
builder.add_node("source", ProducerNode::new("source", producer))?;
builder.add_node("mapper", TransformerNode::new("mapper", transformer))?;
builder.add_node("sink", ConsumerNode::new("sink", consumer))?;
builder.connect(("source", 0), ("mapper", 0))?;
builder.connect(("mapper", 0), ("sink", 0))?;
let graph = builder.build()?;

// DSL (concise)
let graph = graph! {
    source: ProducerNode::new("source", producer)
    => mapper: TransformerNode::new("mapper", transformer)
    => sink: ConsumerNode::new("sink", consumer)
};
```

### Syntax Grammar

```
graph := 'graph!' '{' graph_body '}'
graph_body := (node_def | connection)*
node_def := ident ':' node_expr
connection := node_port arrow node_port
node_port := ident ('.' port_index)?
port_index := integer | ident
arrow := '=>' | '->'
```

### Detailed Syntax

#### Basic Linear Graph

```rust
graph! {
    source: ProducerNode::new("source", producer)
    => mapper: TransformerNode::new("mapper", transformer)
    => sink: ConsumerNode::new("sink", consumer)
}
```

#### Fan-Out (Broadcast)

```rust
graph! {
    source: ProducerNode::new("source", producer)
    => mapper1: TransformerNode::new("mapper1", transformer1)
    => mapper2: TransformerNode::new("mapper2", transformer2)
    => sink: ConsumerNode::new("sink", consumer)
    
    source => mapper1
    source => mapper2
    mapper1 => sink
    mapper2 => sink
}
```

#### Fan-In (Merge)

```rust
graph! {
    source1: ProducerNode::new("source1", producer1)
    source2: ProducerNode::new("source2", producer2)
    merger: TransformerNode::new("merger", merge_transformer)
    sink: ConsumerNode::new("sink", consumer)
    
    source1 => merger
    source2 => merger
    merger => sink
}
```

#### Explicit Ports

```rust
graph! {
    source: ProducerNode::new("source", producer)
    splitter: TransformerNode::new("splitter", split_transformer)
    sink1: ConsumerNode::new("sink1", consumer1)
    sink2: ConsumerNode::new("sink2", consumer2)
    
    source => splitter
    splitter.0 => sink1  // Port 0 to sink1
    splitter.1 => sink2  // Port 1 to sink2
}
```

#### Named Ports

```rust
graph! {
    source: ProducerNode::new("source", producer)
    router: TransformerNode::new("router", router_transformer)
    sink1: ConsumerNode::new("sink1", consumer1)
    sink2: ConsumerNode::new("sink2", consumer2)
    
    source => router
    router.out0 => sink1
    router.out1 => sink2
}
```

#### With Configuration

```rust
graph! {
    source: ProducerNode::new("source", producer)
    => mapper: TransformerNode::new("mapper", transformer)
    => sink: ConsumerNode::new("sink", consumer)
    
    config {
        error_strategy: ErrorStrategy::Skip,
        execution_mode: ExecutionMode::InProcess,
    }
}
```

#### Complex Topology

```rust
graph! {
    // Define nodes
    source: ProducerNode::new("source", producer)
    filter: TransformerNode::new("filter", filter_transformer)
    mapper1: TransformerNode::new("mapper1", mapper1)
    mapper2: TransformerNode::new("mapper2", mapper2)
    aggregator: TransformerNode::new("aggregator", aggregator)
    sink: ConsumerNode::new("sink", consumer)
    
    // Define connections
    source => filter
    filter => mapper1
    filter => mapper2
    mapper1 => aggregator
    mapper2 => aggregator
    aggregator => sink
}
```

### AST Structure

```rust
struct GraphMacro {
    nodes: Vec<NodeDef>,
    connections: Vec<Connection>,
    config: Option<GraphConfig>,
}

struct NodeDef {
    name: Ident,
    expr: Expr,
}

struct Connection {
    source: NodePort,
    target: NodePort,
}

struct NodePort {
    node: Ident,
    port: Option<PortSpec>,
}

enum PortSpec {
    Index(usize),
    Name(Ident),
}

struct GraphConfig {
    error_strategy: Option<Expr>,
    execution_mode: Option<Expr>,
}
```

### Generated Code

The macro expands to:

```rust
// Input:
graph! {
    source: ProducerNode::new("source", producer)
    => mapper: TransformerNode::new("mapper", transformer)
    => sink: ConsumerNode::new("sink", consumer)
}

// Output:
{
    let mut builder = GraphBuilder::new();
    builder.add_node("source".to_string(), ProducerNode::new("source", producer))?;
    builder.add_node("mapper".to_string(), TransformerNode::new("mapper", transformer))?;
    builder.add_node("sink".to_string(), ConsumerNode::new("sink", consumer))?;
    builder.connect(("source".to_string(), 0), ("mapper".to_string(), 0))?;
    builder.connect(("mapper".to_string(), 0), ("sink".to_string(), 0))?;
    builder.build()?
}
```

### Type Safety

The graph DSL maintains type safety through:
- Compile-time node type tracking
- Port type validation
- Connection type compatibility checks
- All validated via existing `GraphBuilder` type system

---

## Implementation Strategy

### Project Structure

```
packages/streamweave-macros/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── pipeline.rs        # Pipeline DSL macro
│   ├── graph.rs           # Graph DSL macro
│   ├── ast.rs             # AST structures
│   ├── parser.rs          # Parsing logic
│   └── codegen.rs         # Code generation
└── tests/
    ├── pipeline.rs
    └── graph.rs
```

### Dependencies

```toml
[lib]
proc-macro = true

[dependencies]
syn = { version = "2.0", features = ["full", "parsing", "extra-traits"] }
proc-macro2 = "1.0"
quote = "1.0"
```

### Implementation Phases

#### Phase 1: Pipeline DSL (MVP)

1. **Parser** (`parser.rs`)
   - Parse `pipeline! { ... }` syntax
   - Extract producer, transformers, consumer
   - Parse optional configuration

2. **Code Generator** (`codegen.rs`)
   - Generate `PipelineBuilder` calls
   - Handle async transformer calls
   - Inject error strategies

3. **Tests**
   - Unit tests for parser
   - Integration tests for generated code
   - Error message tests

#### Phase 2: Graph DSL (Basic)

1. **Parser**
   - Parse node definitions
   - Parse connections
   - Parse port specifications

2. **Code Generator**
   - Generate `GraphBuilder` calls
   - Handle port indices/names
   - Generate connection calls

3. **Tests**
   - Linear graphs
   - Fan-out patterns
   - Fan-in patterns

#### Phase 3: Graph DSL (Advanced)

1. **Enhanced Parser**
   - Named ports
   - Complex topologies
   - Configuration blocks

2. **Validation**
   - Node existence checks
   - Port validation
   - Type compatibility

3. **Error Messages**
   - Clear error messages
   - Source location tracking
   - Helpful suggestions

### Parser Implementation

```rust
// pipeline.rs
use proc_macro::TokenStream;
use syn::{parse_macro_input, Expr};
use quote::quote;

#[proc_macro]
pub fn pipeline(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as PipelineMacro);
    let expanded = generate_pipeline_code(&ast);
    TokenStream::from(expanded)
}

struct PipelineMacro {
    producer: Expr,
    transformers: Vec<Expr>,
    consumer: Expr,
    config: Option<PipelineConfig>,
}

impl syn::parse::Parse for PipelineMacro {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Parse producer
        let producer = input.parse()?;
        
        // Parse transformers (=> transformer)*
        let mut transformers = Vec::new();
        while input.peek(syn::Token![=>]) {
            input.parse::<syn::Token![=>]>()?;
            transformers.push(input.parse()?);
        }
        
        // Parse consumer
        input.parse::<syn::Token![=>]>()?;
        let consumer = input.parse()?;
        
        // Parse optional config
        let config = if input.peek(syn::Token![with]) || input.peek(syn::ident) {
            Some(input.parse()?)
        } else {
            None
        };
        
        Ok(PipelineMacro {
            producer,
            transformers,
            consumer,
            config,
        })
    }
}
```

### Code Generation

```rust
// codegen.rs
use quote::quote;

pub fn generate_pipeline_code(ast: &PipelineMacro) -> proc_macro2::TokenStream {
    let producer = &ast.producer;
    let consumer = &ast.consumer;
    
    // Generate builder chain
    let mut builder_chain = quote! {
        streamweave_pipeline::PipelineBuilder::new()
            .producer(#producer)
    };
    
    // Add transformers
    for transformer in &ast.transformers {
        builder_chain = quote! {
            #builder_chain
                .transformer(#transformer)
                .await
        };
    }
    
    // Add consumer
    builder_chain = quote! {
        #builder_chain
            .consumer(#consumer)
    };
    
    // Add error strategy if present
    if let Some(config) = &ast.config {
        if let Some(error_strategy) = &config.error_strategy {
            builder_chain = quote! {
                #builder_chain
                    .with_error_strategy(#error_strategy)
            };
        }
    }
    
    quote! {
        {
            #builder_chain
        }
    }
}
```

---

## Examples and Use Cases

### Pipeline Examples

#### Simple ETL Pipeline

```rust
use streamweave_macros::pipeline;

let pipeline = pipeline! {
    FileProducer::new("input.csv")
    => CsvParser::new()
    => MapTransformer::new(|row| row.to_uppercase())
    => FileConsumer::new("output.csv")
};
```

#### Error Handling

```rust
let pipeline = pipeline! {
    KafkaProducer::new("topic")
    => RetryTransformer::new(3)
    => DatabaseConsumer::new(connection)
    with ErrorStrategy::Retry(5)
};
```

#### Multiple Transformations

```rust
let pipeline = pipeline! {
    ArrayProducer::new(data)
    => FilterTransformer::new(|x| x > 0)
    => MapTransformer::new(|x| x * 2)
    => SortTransformer::new()
    => VecConsumer::new()
};
```

### Graph Examples

#### Simple Linear Graph

```rust
use streamweave_macros::graph;

let graph = graph! {
    source: ProducerNode::new("source", producer)
    => mapper: TransformerNode::new("mapper", transformer)
    => sink: ConsumerNode::new("sink", consumer)
};
```

#### Parallel Processing

```rust
let graph = graph! {
    source: ProducerNode::new("source", producer)
    processor1: TransformerNode::new("processor1", transformer1)
    processor2: TransformerNode::new("processor2", transformer2)
    aggregator: TransformerNode::new("aggregator", aggregator)
    sink: ConsumerNode::new("sink", consumer)
    
    source => processor1
    source => processor2
    processor1 => aggregator
    processor2 => aggregator
    aggregator => sink
};
```

#### Load Balancing

```rust
let graph = graph! {
    source: ProducerNode::new("source", producer)
    worker1: TransformerNode::new("worker1", worker)
    worker2: TransformerNode::new("worker2", worker)
    worker3: TransformerNode::new("worker3", worker)
    sink: ConsumerNode::new("sink", consumer)
    
    source => worker1
    source => worker2
    source => worker3
    worker1 => sink
    worker2 => sink
    worker3 => sink
};
```

#### Complex Data Flow

```rust
let graph = graph! {
    // Sources
    kafka_source: ProducerNode::new("kafka", kafka_producer)
    file_source: ProducerNode::new("file", file_producer)
    
    // Processing
    validator: TransformerNode::new("validator", validator)
    enricher: TransformerNode::new("enricher", enricher)
    splitter: TransformerNode::new("splitter", splitter)
    
    // Sinks
    db_sink: ConsumerNode::new("db", db_consumer)
    log_sink: ConsumerNode::new("log", log_consumer)
    
    // Connections
    kafka_source => validator
    file_source => validator
    validator => enricher
    enricher => splitter
    splitter.0 => db_sink
    splitter.1 => log_sink
};
```

---

## Migration Path

### Backward Compatibility

The macros are **additive** - they don't replace existing APIs:

```rust
// Old way (still works)
let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .consumer(consumer);

// New way (optional)
let pipeline = pipeline! {
    producer => transformer => consumer
};
```

### Gradual Adoption

1. **Phase 1**: Add macros alongside existing APIs
2. **Phase 2**: Update documentation with macro examples
3. **Phase 3**: Encourage macro usage in new code
4. **Phase 4**: Keep both APIs indefinitely (no breaking changes)

### Feature Flags

```toml
[dependencies]
streamweave-macros = { version = "0.3.0", optional = true }

[features]
default = []
macros = ["streamweave-macros"]
```

Users can opt-in:
```rust
#[cfg(feature = "macros")]
use streamweave_macros::pipeline;
```

---

## Future Enhancements

### 1. Visual Graph Generation

Generate visual representations from DSL:

```rust
let graph = graph! { ... };
graph.visualize("output.dot");  // Generate Graphviz
```

### 2. Graph Serialization

Serialize graphs to JSON/YAML:

```rust
let graph = graph! { ... };
let json = graph.to_json();
let loaded = Graph::from_json(json);
```

### 3. Graph Templates

Reusable graph patterns:

```rust
macro_rules! etl_pipeline {
    ($source:expr, $transform:expr, $sink:expr) => {
        graph! {
            source: $source
            => transform: $transform
            => sink: $sink
        }
    };
}
```

### 4. IDE Support

- Autocomplete for node types
- Type checking in IDE
- Graph visualization in IDE
- Refactoring support

### 5. Advanced Features

- Conditional connections
- Dynamic node creation
- Subgraph support
- Graph composition

---

## Conclusion

The macro-based DSL for StreamWeave provides:

1. **Conciseness**: 70% reduction in boilerplate
2. **Type Safety**: Full compile-time validation
3. **Expressiveness**: Support for complex topologies
4. **Backward Compatibility**: Works alongside existing APIs
5. **Future-Proof**: Extensible for advanced features

The DSL enhances developer experience while maintaining all the benefits of StreamWeave's type-safe architecture.

