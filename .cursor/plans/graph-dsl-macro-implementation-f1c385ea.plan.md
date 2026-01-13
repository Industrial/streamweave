<!-- f1c385ea-8f0b-4992-abab-2bebbeed7b7c 026fa6dc-1e83-4918-aeaa-6d896a49091d -->
# Graph DSL Macro Implementation Plan

## Overview

Create a procedural macro `graph!` that provides a concise DSL for defining graph topologies. The macro will:

- Parse connection syntax: `node1:out -> node2:in`
- Validate port names at compile time
- Generate `GraphBuilder` API calls
- Support comments in the DSL

## Architecture Changes

### 1. Refactor Port System for Compile-Time Validation

**File: `src/graph/traits.rs`**

Add associated constants to `NodeTrait` for compile-time port name access:

```rust
pub trait NodeTrait: Send + Sync + std::any::Any + DynClone {
    // Existing methods...
    
    /// Compile-time input port names for macro validation
    const INPUT_PORTS: &'static [&'static str];
    
    /// Compile-time output port names for macro validation  
    const OUTPUT_PORTS: &'static [&'static str];
}
```

**Impact:** All node implementations must define these constants.

### 2. Update Node Implementations

**Files to update:**

- `src/graph/nodes/node.rs` (ProducerNode, TransformerNode, ConsumerNode)
- All custom node implementations in `src/graph/nodes/`

**Pattern for each node type:**

```rust
impl<P, Outputs> NodeTrait for ProducerNode<P, Outputs> {
    const INPUT_PORTS: &'static [&'static str] = &[]; // Producers have no inputs
    const OUTPUT_PORTS: &'static [&'static str] = &["out"]; // or ["out_0", "out_1", ...]
    // ... existing implementations
}
```

**Default port naming:**

- Single port: `["out"]` or `["in"]`
- Multiple ports: `["out_0", "out_1", ...]` or `["in_0", "in_1", ...]`

### 3. Create DSL Module Structure

**New directory: `src/graph/dsl/`**

```
src/graph/dsl/
├── mod.rs              # Public API, re-exports
├── macro_impl.rs       # Procedural macro definition
├── parser.rs            # Syntax parsing
├── validator.rs         # Compile-time validation
├── codegen.rs           # GraphBuilder code generation
└── error.rs             # DSL-specific error types
```

### 4. Procedural Macro Implementation

**File: `src/graph/dsl/macro_impl.rs`**

**Macro syntax:**

```rust
graph! {
    // Comments supported
    source:out -> transform:in,
    transform:out -> sink:in,
}
```

**Macro expansion process:**

1. Parse input tokens into AST
2. Extract node references and connections
3. Validate port names at compile time using `NodeTrait::INPUT_PORTS` and `NodeTrait::OUTPUT_PORTS`
4. Generate `GraphBuilder` code
5. Return `Result<Graph, String>` (compile-time errors become compile errors)

**Generated code structure:**

```rust
{
    let mut builder = GraphBuilder::new();
    builder = builder.node(source)?;
    builder = builder.node(transform)?;
    builder = builder.node(sink)?;
    builder = builder.connect("source", "out", "transform", "in")?;
    builder = builder.connect("transform", "out", "sink", "in")?;
    builder.build()
}
```

### 5. Compile-Time Validation

**File: `src/graph/dsl/validator.rs`**

**Validation steps:**

1. **Node existence:** Verify all referenced nodes are in scope and implement `NodeTrait`
2. **Port existence:** Check port names against `NodeTrait::INPUT_PORTS` and `NodeTrait::OUTPUT_PORTS`
3. **Port direction:** Verify output ports connect to input ports
4. **Node names:** Extract node names from node instances (via `NodeTrait::name()` at compile time if possible, or require explicit names)

**Validation approach:**

- Use `quote!` to generate validation code that runs at compile time
- For port validation, generate code that checks against const arrays
- Emit compile errors using `syn::Error` for invalid syntax

**Example validation code generation:**

```rust
// For connection: source:out -> transform:in
{
    // Validate source has output port "out"
    const _: () = {
        const PORTS: &[&str] = <SourceNodeType as NodeTrait>::OUTPUT_PORTS;
        assert!(PORTS.contains(&"out"), "Port 'out' not found on source node");
    };
    // Similar for transform input port "in"
}
```

### 6. Parser Implementation

**File: `src/graph/dsl/parser.rs`**

**Parse connection syntax:**

- Pattern: `node_ident:port_name -> target_ident:port_name`
- Support: `node_ident -> target_ident` (default ports - first output to first input)
- Support: Comments (`//` style)

**AST structure:**

```rust
pub struct Connection {
    pub source: NodeRef,
    pub source_port: Option<String>,
    pub target: NodeRef,
    pub target_port: Option<String>,
}

pub struct NodeRef {
    pub ident: syn::Ident,
}
```

**Parsing strategy:**

- Use `syn::parse_macro_input!` for macro input
- Parse as a block with connection statements
- Handle comma/semicolon separators
- Strip comments

### 7. Code Generation

**File: `src/graph/dsl/codegen.rs`**

**Code generation steps:**

1. Collect all unique node references
2. Generate node addition calls: `builder.node(node_ident)?`
3. Generate connection calls: `builder.connect(source_name, source_port, target_name, target_port)?`
4. Extract node names (requires runtime call to `node.name()` or explicit name binding)

**Node name extraction challenge:**

- Nodes have names set at creation time
- Macro needs node names for `connect()` calls
- **Solution:** Generate code that calls `node.name()` at runtime, or require nodes to be bound with known names

**Approach:** Generate code that extracts names at runtime:

```rust
let source_name = source.name();
let transform_name = transform.name();
// Use in connect() calls
```

### 8. Error Handling

**File: `src/graph/dsl/error.rs`**

**Error types:**

- Parse errors (invalid syntax)
- Validation errors (invalid ports, missing nodes)
- Type errors (nodes don't implement NodeTrait)

**Error messages:**

- Use `syn::Error` for compile-time errors
- Provide helpful suggestions
- Include line numbers and context

### 9. Integration

**File: `src/graph/mod.rs`**

Export the macro:

```rust
pub use dsl::graph;
```

**File: `src/graph/dsl/mod.rs`**

Define the macro module and re-export:

```rust
mod macro_impl;
mod parser;
mod validator;
mod codegen;
mod error;

pub use macro_impl::graph;
```

**File: `Cargo.toml`**

Add proc-macro support to the main crate:

```toml
[lib]
proc-macro = true

[dependencies]
syn = { version = "2.0", features = ["full", "parsing", "extra-traits"] }
quote = "1.0"
proc-macro2 = "1.0"
```

### 10. Testing Strategy

**Test files:**

- `src/graph/dsl/tests.rs` - Unit tests for DSL usage and macro expansion

**Test cases:**

1. Simple linear graph
2. Fan-out pattern
3. Fan-in pattern
4. Complex topology
5. Invalid port names (compile error)
6. Missing nodes (compile error)
7. Comments in DSL
8. Default ports (no port notation)

## Implementation Steps

### Phase 1: Refactor Port System

1. Add `INPUT_PORTS` and `OUTPUT_PORTS` constants to `NodeTrait`
2. Update `ProducerNode`, `TransformerNode`, `ConsumerNode` implementations
3. Update all custom node implementations
4. Add tests to verify constants match runtime port names

### Phase 2: Create DSL Module

1. Create `src/graph/dsl/` directory structure
2. Set up `Cargo.toml` with proc-macro support
3. Create basic macro skeleton that parses and expands

### Phase 3: Implement Parser

1. Parse connection syntax
2. Handle comments
3. Extract node references and port names
4. Build AST

### Phase 4: Implement Validator

1. Validate node references exist
2. Validate port names against const arrays
3. Generate compile-time validation code
4. Emit helpful error messages

### Phase 5: Implement Code Generator

1. Generate GraphBuilder initialization
2. Generate node addition calls
3. Generate connection calls with port names
4. Handle node name extraction

### Phase 6: Integration & Testing

1. Export macro from graph module
2. Write comprehensive tests
3. Update documentation
4. Create examples

## Files to Create/Modify

### New Files

- `src/graph/dsl/mod.rs` - Module definition and re-exports
- `src/graph/dsl/macro.rs` - Procedural macro definition
- `src/graph/dsl/parser.rs` - Syntax parsing
- `src/graph/dsl/validator.rs` - Compile-time validation
- `src/graph/dsl/codegen.rs` - GraphBuilder code generation
- `src/graph/dsl/error.rs` - DSL-specific error types
- `src/graph/dsl/tests.rs` - Unit tests for DSL usage

### Modified Files

- `src/graph/traits.rs` - Add const port name constants
- `src/graph/nodes/node.rs` - Implement constants for all node types
- `src/graph/mod.rs` - Export DSL module and macro
- `Cargo.toml` - Add proc-macro support and dependencies
- All custom node implementations - Add port name constants

## Key Design Decisions

1. **Port names as constants:** Enables compile-time validation
2. **Runtime node name extraction:** Simpler than requiring explicit names
3. **Default ports:** Support `node1 -> node2` syntax for convenience
4. **Comments:** Support `//` style comments for documentation

## Success Criteria

- [ ] Macro parses connection syntax correctly
- [ ] Compile-time validation catches invalid port names
- [ ] Generated code uses GraphBuilder API correctly
- [ ] All existing node types have port name constants
- [ ] Tests verify macro expansion and validation
- [ ] Documentation includes DSL syntax and examples