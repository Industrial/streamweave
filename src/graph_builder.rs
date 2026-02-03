//! # GraphBuilder
//!
//! Builder pattern for constructing graphs with a fluent API.
//!
//! The `GraphBuilder` provides a convenient way to build graphs programmatically,
//! allowing you to add nodes, create connections, and set up graph I/O bindings
//! before constructing the final `Graph` instance.

use crate::edge::Edge;
use crate::graph::Graph;
use crate::graph::GraphExecutionError;
use crate::node::Node;
use std::any::Any;
use std::sync::Arc;

/// Builder for constructing graphs with a fluent API.
///
/// `GraphBuilder` allows you to incrementally build a graph by:
/// - Adding nodes with explicit names
/// - Creating connections between node ports
/// - Binding graph inputs and outputs
///
/// Once construction is complete, call `build()` to create the final `Graph`.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::graph::builder::GraphBuilder;
/// use streamweave::node::Node;
///
/// let graph = GraphBuilder::new("my_graph")
///     .add_node("source", Box::new(source_node))
///     .connect("source", "out", "sink", "in")
///     .build()?;
/// ```
pub struct GraphBuilder {
    /// The name of the graph being built.
    name: String,
    /// Nodes to add to the graph: (name, node_instance)
    nodes: Vec<(String, Box<dyn Node>)>,
    /// Edges connecting nodes.
    edges: Vec<Edge>,
    /// Input bindings: (external_name, node_name, port_name, optional_value)
    input_bindings: Vec<(String, String, String, Option<Arc<dyn Any + Send + Sync>>)>,
    /// Output bindings: (external_name, node_name, port_name)
    output_bindings: Vec<(String, String, String)>,
}

impl GraphBuilder {
    /// Creates a new `GraphBuilder` with the given graph name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name for the graph being built
    ///
    /// # Returns
    ///
    /// A new `GraphBuilder` instance ready to build a graph.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use streamweave::graph::builder::GraphBuilder;
    ///
    /// let builder = GraphBuilder::new("my_graph");
    /// ```
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            input_bindings: Vec::new(),
            output_bindings: Vec::new(),
        }
    }

    /// Adds a node to the graph being built.
    ///
    /// # Arguments
    ///
    /// * `name` - The name for the node (must be unique within the graph)
    /// * `node` - The node instance to add
    ///
    /// # Returns
    ///
    /// Returns `self` for method chaining.
    ///
    /// # Panics
    ///
    /// Panics if the node name is `"graph"`, which is reserved for graph I/O namespace.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use streamweave::graph::builder::GraphBuilder;
    /// use streamweave::node::Node;
    ///
    /// let builder = GraphBuilder::new("my_graph")
    ///     .add_node("source", Box::new(source_node));
    /// ```
    pub fn add_node(mut self, name: impl Into<String>, node: Box<dyn Node>) -> Self {
        let name_str = name.into();
        if name_str == "graph" {
            panic!(
                "Node name 'graph' is reserved for graph I/O namespace. Use 'graph.input_name' for graph inputs and 'graph.output_name' for graph outputs."
            );
        }
        self.nodes.push((name_str, node));
        self
    }

    /// Creates a connection between two node ports.
    ///
    /// # Arguments
    ///
    /// * `source` - The name of the source node
    /// * `source_port` - The name of the output port on the source node
    /// * `target` - The name of the target node
    /// * `target_port` - The name of the input port on the target node
    ///
    /// # Returns
    ///
    /// Returns `self` for method chaining.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use streamweave::graph::builder::GraphBuilder;
    ///
    /// let builder = GraphBuilder::new("my_graph")
    ///     .add_node("source", Box::new(source_node))
    ///     .add_node("sink", Box::new(sink_node))
    ///     .connect("source", "out", "sink", "in");
    /// ```
    pub fn connect(mut self, source: &str, source_port: &str, target: &str, target_port: &str) -> Self {
        // Check for fan-out: same source port already connected
        if self.edges.iter().any(|e| e.source_node() == source && e.source_port() == source_port) {
            panic!(
                "Fan-out not supported: output port '{}.{}' is already connected. Each output port can only connect to one input port.",
                source, source_port
            );
        }
        
        self.edges.push(Edge {
            source_node: source.to_string(),
            source_port: source_port.to_string(),
            target_node: target.to_string(),
            target_port: target_port.to_string(),
        });
        self
    }

    /// Binds a graph input port to an internal node's input port.
    ///
    /// This method allows you to expose an internal node's input port as a graph-level
    /// input port. Optionally, you can provide an initial value that will be sent to
    /// the node when the graph is built.
    ///
    /// # Arguments
    ///
    /// * `external_name` - The name of the external graph input port
    /// * `node` - The name of the internal node
    /// * `port` - The name of the input port on the internal node
    /// * `value` - Optional initial value to send to the node (if `Some`, the value
    ///   will be sent via a channel when the graph is built)
    ///
    /// # Returns
    ///
    /// Returns `self` for method chaining.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use streamweave::graph::builder::GraphBuilder;
    ///
    /// // Bind graph input with an initial value
    /// let builder = GraphBuilder::new("my_graph")
    ///     .add_node("source", Box::new(source_node))
    ///     .input("start", "source", "start", Some(1i32));
    ///
    /// // Bind graph input without a value (for runtime connection)
    /// let builder = GraphBuilder::new("my_graph")
    ///     .add_node("source", Box::new(source_node))
    ///     .input("config", "source", "configuration", None::<()>);
    /// ```
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

    /// Binds a graph output port to an internal node's output port.
    ///
    /// This method allows you to expose an internal node's output port as a graph-level
    /// output port, making it accessible from outside the graph.
    ///
    /// # Arguments
    ///
    /// * `external_name` - The name of the external graph output port
    /// * `node` - The name of the internal node
    /// * `port` - The name of the output port on the internal node
    ///
    /// # Returns
    ///
    /// Returns `self` for method chaining.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use streamweave::graph::builder::GraphBuilder;
    ///
    /// let builder = GraphBuilder::new("my_graph")
    ///     .add_node("sink", Box::new(sink_node))
    ///     .output("result", "sink", "out");
    /// ```
    pub fn output(mut self, external_name: impl Into<String>, node: &str, port: &str) -> Self {
        self.output_bindings.push((
            external_name.into(),
            node.to_string(),
            port.to_string(),
        ));
        self
    }

    /// Builds the final `Graph` instance from the builder.
    ///
    /// This method:
    /// 1. Creates a new `Graph` with the specified name
    /// 2. Adds all nodes that were added via `add_node()`
    /// 3. Creates all edges that were added via `connect()`
    /// 4. Exposes input ports and connects channels for values (if provided)
    /// 5. Exposes output ports
    ///
    /// # Returns
    ///
    /// `Ok(Graph)` if the graph was built successfully, or an error if:
    /// - A node with the same name already exists
    /// - An edge references a non-existent node or port
    /// - An input/output binding references a non-existent node or port
    ///
    /// # Errors
    ///
    /// Returns `GraphExecutionError` if the graph cannot be built.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use streamweave::graph::builder::GraphBuilder;
    ///
    /// let graph = GraphBuilder::new("my_graph")
    ///     .add_node("source", Box::new(source_node))
    ///     .add_node("sink", Box::new(sink_node))
    ///     .connect("source", "out", "sink", "in")
    ///     .input("start", "source", "start", Some(1i32))
    ///     .output("result", "sink", "out")
    ///     .build()?;
    /// ```
    pub fn build(self) -> Result<Graph, GraphExecutionError> {
        let mut graph = Graph::new(self.name);

        // Add nodes
        for (name, node) in self.nodes {
            graph.add_node(name.clone(), node)
                .map_err(|e: String| -> GraphExecutionError { Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) })?;
        }

        // Add edges
        for edge in self.edges {
            graph.add_edge(edge)
                .map_err(|e: String| -> GraphExecutionError { Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) })?;
        }

        // Setup input bindings
        for (external, node, port, value) in self.input_bindings {
            graph.expose_input_port(&node, &port, &external)
                .map_err(|e: String| -> GraphExecutionError { Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) })?;

            // If a value was provided, create a channel and send it
            if let Some(val) = value {
                let (tx, rx) = tokio::sync::mpsc::channel(1);
                graph.connect_input_channel(&external, rx)
                    .map_err(|e: String| -> GraphExecutionError { Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) })?;
                tokio::spawn(async move {
                    let _ = tx.send(val).await;
                });
            }
        }

        // Setup output bindings
        for (external, node, port) in self.output_bindings {
            graph.expose_output_port(&node, &port, &external)
                .map_err(|e: String| -> GraphExecutionError { Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) })?;
        }

        Ok(graph)
    }
}
