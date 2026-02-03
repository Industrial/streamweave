//! # Graph Macros
//!
//! Macros for declarative graph construction with minimal syntax.
//!
//! This module provides macros that allow you to define graphs using a concise,
//! declarative syntax, reducing boilerplate and improving readability.

/// Declarative macro for constructing graphs with minimal syntax.
///
/// The `graph!` macro allows you to define graphs using a concise, declarative syntax
/// that reduces boilerplate and improves readability compared to the traditional Graph API.
///
/// # Syntax
///
/// ```rust,no_run
/// use streamweave::graph;
///
/// let graph = graph! {
///     node1: Node1::new("node1"),
///     node2: Node2::new("node2"),
///     // Optional connections section
///     ; node1.out => node2.in
/// };
/// ```
///
/// # Node Definitions
///
/// Nodes are defined using the pattern `name: NodeInstance`, where:
/// - `name` is an identifier that will be used to reference the node in connections
/// - `NodeInstance` is an expression that evaluates to a node instance
/// - Multiple nodes are separated by commas
///
/// # Connections
///
/// Connections are optional and specified after a semicolon. Supported patterns:
///
/// ## Node-to-Node Connections
///
/// ```rust,no_run
/// # use streamweave::graph;
/// graph! {
///     source: SourceNode::new(),
///     sink: SinkNode::new(),
///     ; source.out => sink.in
/// };
/// ```
///
/// All ports must be explicitly named. No default ports or sequential shortcuts.
///
/// ## Graph Inputs
///
/// ### With Initial Value
///
/// ```rust,no_run
/// # use streamweave::graph;
/// graph! {
///     node: Node::new(),
///     graph.config: 42i32 => node.in
/// };
/// ```
///
/// The value is sent via a channel when the graph is built.
///
/// ### Without Value (Runtime Connection)
///
/// ```rust,no_run
/// # use streamweave::graph;
/// graph! {
///     node: Node::new(),
///     ; graph.input => node.in
/// };
/// ```
///
/// External channels must be connected at runtime.
///
/// ## Graph Outputs
///
/// ```rust,no_run
/// # use streamweave::graph;
/// graph! {
///     node: Node::new(),
///     node.out => graph.output
/// };
/// ```
///
/// # Restrictions
///
/// - **Reserved Node Name**: Node names cannot be `"graph"` - this name is reserved for graph I/O namespace
/// - **Unique Node Names**: All node names must be unique within the graph
/// - **Fan-Out Not Supported**: Each output port can only connect to one input port. Attempting to connect
///   the same source port to multiple targets will panic at build time.
/// - **Fan-In Allowed**: Multiple sources can connect to the same target port (fan-in is supported)
/// - **Explicit Ports Required**: All connections must explicitly specify port names on both source and target sides
///
/// # Examples
///
/// ## Simple Linear Pipeline
///
/// ```rust,no_run
/// # use streamweave::graph;
/// let graph = graph! {
///     producer: ProducerNode::new(),
///     transform: TransformNode::new(),
///     sink: SinkNode::new(),
///     producer.out => transform.in,
///     transform.out => sink.in
/// };
/// ```
///
/// ## Graph I/O with Values
///
/// ```rust,no_run
/// # use streamweave::graph;
/// let graph = graph! {
///     transform: TransformNode::new(),
///     graph.config: 100i32 => transform.in,
///     transform.out => graph.result
/// };
/// ```
///
/// ## Fan-In Pattern
///
/// ```rust,no_run
/// # use streamweave::graph;
/// let graph = graph! {
///     source1: SourceNode::new(),
///     source2: SourceNode::new(),
///     merge: MergeNode::new(),
///     ; source1.out => merge.in1,
///       source2.out => merge.in2
/// };
/// ```
///
/// # Error Handling
///
/// The macro will panic at build time if:
/// - A node is named `"graph"` (compile-time error via `validate_node_name!`)
/// - Fan-out is detected (runtime panic in `GraphBuilder::connect`)
/// - Invalid ports or nodes are referenced (errors from `Graph::add_edge`)
///
/// # See Also
///
/// - [`GraphBuilder`](crate::graph_builder::GraphBuilder) - Fluent API alternative
/// - [`Graph`](crate::graph::Graph) - Core graph type
/// - Examples in `examples/` directory
#[macro_export]
macro_rules! graph {
    // Parse nodes and connections in a single pass (no semicolon needed)
    // Nodes: name: expr
    // Connections: source.port => target.port (or graph I/O patterns)
    (
        $($items:tt)*
    ) => {{
        let builder = $crate::graph_builder::GraphBuilder::new("graph");
        $crate::graph_parse_items!(builder, $($items)*);
        builder.build().expect("Failed to build graph")
    }};
}

/// Helper macro to parse items (nodes and connections mixed).
///
/// This macro recursively processes items one at a time, distinguishing between
/// node definitions (`name: expr`) and connection patterns (`source.port => target.port`).
///
/// This macro is used internally by the `graph!` macro and should not be called directly.
#[macro_export]
macro_rules! graph_parse_items {
    // Base case: done
    ($builder:ident $(,)?) => {};

    // Node pattern: name: expr, ...
    ($builder:ident, $name:ident : $node:expr, $($rest:tt)*) => {
        $crate::validate_node_name!(stringify!($name));
        let $builder = $builder.add_node(stringify!($name), Box::new($node));
        $crate::graph_parse_items!($builder, $($rest)*);
    };

    // Node pattern without trailing comma: name: expr
    ($builder:ident, $name:ident : $node:expr) => {
        $crate::validate_node_name!(stringify!($name));
        let $builder = $builder.add_node(stringify!($name), Box::new($node));
    };

    // Graph input with value: graph.name: value => node.port, ...
    // Must come before node pattern to avoid matching "graph" as a node name
    ($builder:ident, graph . $in_name:ident : $value:expr => $node:ident . $port:ident, $($rest:tt)*) => {
        let $builder = $builder.input(stringify!($in_name), stringify!($node), stringify!($port), Some($value));
        $crate::graph_parse_items!($builder, $($rest)*);
    };

    // Graph input with value without trailing comma
    ($builder:ident, graph . $in_name:ident : $value:expr => $node:ident . $port:ident) => {
        let $builder = $builder.input(stringify!($in_name), stringify!($node), stringify!($port), Some($value));
    };

    // Graph input without value: graph.name => node.port, ...
    // Must come before node pattern to avoid matching "graph" as a node name
    ($builder:ident, graph . $in_name:ident => $node:ident . $port:ident, $($rest:tt)*) => {
        let $builder = $builder.input(stringify!($in_name), stringify!($node), stringify!($port), None::<()>);
        $crate::graph_parse_items!($builder, $($rest)*);
    };

    // Graph input without value without trailing comma
    ($builder:ident, graph . $in_name:ident => $node:ident . $port:ident) => {
        let $builder = $builder.input(stringify!($in_name), stringify!($node), stringify!($port), None::<()>);
    };

    // Graph output: node.port => graph.name, ...
    // Must come before node-to-node pattern
    ($builder:ident, $node:ident . $port:ident => graph . $out_name:ident, $($rest:tt)*) => {
        let $builder = $builder.output(stringify!($out_name), stringify!($node), stringify!($port));
        $crate::graph_parse_items!($builder, $($rest)*);
    };

    // Graph output without trailing comma
    ($builder:ident, $node:ident . $port:ident => graph . $out_name:ident) => {
        let $builder = $builder.output(stringify!($out_name), stringify!($node), stringify!($port));
    };

    // Node-to-node connection: source.port => target.port, ...
    // Must come after graph I/O patterns
    ($builder:ident, $source:ident . $sp:ident => $target:ident . $tp:ident, $($rest:tt)*) => {
        let $builder = $builder.connect(stringify!($source), stringify!($sp), stringify!($target), stringify!($tp));
        $crate::graph_parse_items!($builder, $($rest)*);
    };

    // Node-to-node connection without trailing comma
    ($builder:ident, $source:ident . $sp:ident => $target:ident . $tp:ident) => {
        let $builder = $builder.connect(stringify!($source), stringify!($sp), stringify!($target), stringify!($tp));
    };
}

/// Parses connection patterns and generates GraphBuilder method calls.
///
/// This macro is used internally by the `graph!` macro to parse connection syntax
/// and generate the appropriate builder method calls.
///
/// # Supported Patterns
///
/// ## Node-to-Node Connections
///
/// `source_node.source_port => target_node.target_port`
///
/// ```rust,no_run
/// # use streamweave::graph_builder::GraphBuilder;
/// # use streamweave::parse_connections;
/// let mut builder = GraphBuilder::new("test");
/// builder = parse_connections!(builder, node1.out => node2.in);
/// ```
///
/// ## Graph Inputs
///
/// With value: `graph.input_name: value => node.port`
///
/// ```rust,no_run
/// # use streamweave::graph_builder::GraphBuilder;
/// # use streamweave::parse_connections;
/// let mut builder = GraphBuilder::new("test");
/// builder = parse_connections!(builder, graph.config: 42i32 => node.in);
/// ```
///
/// Without value: `graph.input_name => node.port`
///
/// ```rust,no_run
/// # use streamweave::graph_builder::GraphBuilder;
/// # use streamweave::parse_connections;
/// let mut builder = GraphBuilder::new("test");
/// builder = parse_connections!(builder, graph.input => node.in);
/// ```
///
/// ## Graph Outputs
///
/// `node.port => graph.output_name`
///
/// ```rust,no_run
/// # use streamweave::graph_builder::GraphBuilder;
/// # use streamweave::parse_connections;
/// let mut builder = GraphBuilder::new("test");
/// builder = parse_connections!(builder, node.out => graph.result);
/// ```
///
/// # Multiple Connections
///
/// Multiple connections can be specified, separated by commas:
///
/// ```rust,no_run
/// # use streamweave::graph_builder::GraphBuilder;
/// # use streamweave::parse_connections;
/// let mut builder = GraphBuilder::new("test");
/// builder = parse_connections!(
///     builder,
///     node1.out => node2.in,
///     node2.out => node3.in,
///     graph.input => node1.in,
///     node3.out => graph.output
/// );
/// ```
///
/// # Restrictions
///
/// - **Fan-out Not Supported**: Same source port cannot connect to multiple targets
/// - **Fan-in Allowed**: Multiple sources can connect to the same target port
/// - **Rule Ordering**: Graph I/O patterns must be matched before node-to-node patterns
///   (handled internally by macro rule ordering)
///
/// # Usage
///
/// This macro is typically used internally by `graph!`, but can be used directly
/// for programmatic graph construction.
#[macro_export]
macro_rules! parse_connections {
    // Parse graph input with value: graph.input_name: value => node.port
    // Must come before node-to-node rule to avoid matching "graph" as a node name
    ($builder:ident, graph . $input_name:ident : $value:expr => $node:ident . $port:ident $(, $rest:tt)*) => {
        {
            let mut $builder = $builder.input(stringify!($input_name), stringify!($node), stringify!($port), Some($value));
            $crate::parse_connections!($builder, $($rest)*)
        }
    };

    // Parse graph input connection: graph.input_name => node.port
    // Must come before node-to-node rule to avoid matching "graph" as a node name
    ($builder:ident, graph . $input_name:ident => $node:ident . $port:ident $(, $rest:tt)*) => {
        {
            let mut $builder = $builder.input(stringify!($input_name), stringify!($node), stringify!($port), None::<()>);
            $crate::parse_connections!($builder, $($rest)*)
        }
    };

    // Parse graph output: node.port => graph.output_name
    // Must come before node-to-node rule to avoid matching "graph" as a node name
    ($builder:ident, $node:ident . $port:ident => graph . $output_name:ident $(, $rest:tt)*) => {
        {
            let mut $builder = $builder.output(stringify!($output_name), stringify!($node), stringify!($port));
            $crate::parse_connections!($builder, $($rest)*)
        }
    };

    // Parse node-to-node connection: source_node.source_port => target_node.target_port
    // All ports must be explicitly named
    // Fan-out detection happens at runtime in GraphBuilder::connect()
    // Must come after graph I/O rules
    ($builder:ident, $source_node:ident . $source_port:ident => $target_node:ident . $target_port:ident $(, $rest:tt)*) => {
        {
            let mut $builder = $builder.connect(stringify!($source_node), stringify!($source_port), stringify!($target_node), stringify!($target_port));
            $crate::parse_connections!($builder, $($rest)*)
        }
    };

    // Base case: no more connections
    ($builder:ident, $(,)?) => {
        $builder
    };
}

/// Validates that a node name is not "graph".
///
/// This macro is used internally by the `graph!` macro to ensure that no node
/// is named "graph", which is reserved for the graph I/O namespace.
///
/// # Usage
///
/// This macro is typically used internally by `graph!`, but can be used directly:
///
/// ```rust,compile_fail
/// # use streamweave::validate_node_name;
/// validate_node_name!("graph"); // This will cause a compile error
/// ```
///
/// # Errors
///
/// If the node name is "graph", this macro will emit a `compile_error!` with
/// a helpful message explaining that "graph" is reserved for graph I/O namespace.
#[macro_export]
macro_rules! validate_node_name {
    ("graph") => {
        compile_error!(
            "Node name 'graph' is reserved for graph I/O namespace. Use 'graph.input_name' for graph inputs and 'graph.output_name' for graph outputs."
        );
    };
    ($name:expr) => {};
}
