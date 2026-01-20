//! # GraphBuilder - Fluent API for Graph Construction
//!
//! This module provides the `GraphBuilder` struct for constructing StreamWeave graphs
//! using a fluent, type-safe API. It allows chaining method calls to add nodes,
//! connect them, and configure port mappings before building the final Graph.
//!
//! ## Overview
//!
//! GraphBuilder provides:
//!
//! - **Fluent API**: Method chaining for ergonomic graph construction
//! - **Type Safety**: Compile-time validation of graph structure
//! - **Validation**: Runtime validation of node connections and port mappings
//! - **Port Mapping**: Automatic configuration of input/output port mappings
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::{GraphBuilder, nodes::arithmetic::AddNode};
//!
//! let graph = GraphBuilder::new("calculator")
//!     .add_node("add", Box::new(AddNode::new("add".to_string())))
//!     .expose_input_port("add", "in1", "input")
//!     .expose_output_port("add", "out", "output")
//!     .build()
//!     .unwrap();
//! ```

use crate::edge::Edge;
use crate::graph::Graph;
use crate::node::Node;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

/// Builder for constructing graphs with a fluent API.
///
/// GraphBuilder provides a type-safe, fluent interface for constructing
/// StreamWeave graphs. It allows chaining method calls to add nodes,
/// connect them, and configure port mappings before building the final Graph.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::{GraphBuilder, nodes::arithmetic::AddNode};
///
/// let graph = GraphBuilder::new("calculator")
///     .add_node("add", Box::new(AddNode::new("add".to_string())))
///     .expose_input_port("add", "in1", "input")
///     .expose_output_port("add", "out", "output")
///     .build()
///     .unwrap();
/// ```
///
pub struct GraphBuilder {
  /// The name of the graph being built
  name: String,
  /// Nodes to be added to the graph
  nodes: Vec<(String, Box<dyn Node>)>,
  /// Edges to be added to the graph
  edges: Vec<Edge>,
  /// Input port mappings to be configured
  input_mappings: Vec<(String, String, String)>, // (internal_node, internal_port, external_name)
  /// Output port mappings to be configured
  output_mappings: Vec<(String, String, String)>, // (internal_node, internal_port, external_name)
  /// External input channels to connect
  external_input_channels: HashMap<String, tokio::sync::mpsc::Receiver<Arc<dyn Any + Send + Sync>>>, // (external_port, receiver)
  /// External output channels to connect
  external_output_channels: HashMap<String, tokio::sync::mpsc::Sender<Arc<dyn Any + Send + Sync>>>, // (external_port, sender)
}

impl GraphBuilder {
  /// Creates a new GraphBuilder with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name for the graph
  ///
  /// # Returns
  ///
  /// A new `GraphBuilder` instance.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::GraphBuilder;
  ///
  /// let builder = GraphBuilder::new("my_graph");
  /// ```
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      nodes: Vec::new(),
      edges: Vec::new(),
      input_mappings: Vec::new(),
      output_mappings: Vec::new(),
      external_input_channels: HashMap::new(),
      external_output_channels: HashMap::new(),
    }
  }

  /// Adds a node to the graph.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node (must be unique within the graph)
  /// * `node` - The node instance to add
  ///
  /// # Returns
  ///
  /// The builder instance for method chaining.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::{GraphBuilder, nodes::arithmetic::AddNode};
  ///
  /// let builder = GraphBuilder::new("calc")
  ///     .add_node("adder", Box::new(AddNode::new("adder".to_string())));
  /// ```
  pub fn add_node(mut self, name: impl Into<String>, node: Box<dyn Node>) -> Self {
    self.nodes.push((name.into(), node));
    self
  }

  /// Connects two nodes by creating an edge between their ports.
  ///
  /// # Arguments
  ///
  /// * `from_node` - The name of the source node
  /// * `from_port` - The output port name on the source node
  /// * `to_node` - The name of the target node
  /// * `to_port` - The input port name on the target node
  ///
  /// # Returns
  ///
  /// The builder instance for method chaining.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::GraphBuilder;
  ///
  /// let builder = GraphBuilder::new("pipeline")
  ///     .connect("source", "out", "processor", "in")
  ///     .connect("processor", "out", "sink", "in");
  /// ```
  pub fn connect(
    mut self,
    from_node: impl Into<String>,
    from_port: impl Into<String>,
    to_node: impl Into<String>,
    to_port: impl Into<String>,
  ) -> Self {
    self.edges.push(Edge {
      source_node: from_node.into(),
      source_port: from_port.into(),
      target_node: to_node.into(),
      target_port: to_port.into(),
    });
    self
  }

  /// Exposes an internal node's input port as an external input port.
  ///
  /// This allows external streams to flow into the graph through specific internal nodes.
  ///
  /// # Arguments
  ///
  /// * `internal_node` - The name of the internal node
  /// * `internal_port` - The name of the input port on the internal node
  /// * `external_name` - The external port name ("configuration" or "input")
  ///
  /// # Returns
  ///
  /// The builder instance for method chaining.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::GraphBuilder;
  ///
  /// let builder = GraphBuilder::new("service")
  ///     .expose_input_port("handler", "data", "input")
  ///     .expose_input_port("config", "settings", "configuration");
  /// ```
  pub fn expose_input_port(
    mut self,
    internal_node: impl Into<String>,
    internal_port: impl Into<String>,
    external_name: impl Into<String>,
  ) -> Self {
    self.input_mappings.push((
      internal_node.into(),
      internal_port.into(),
      external_name.into(),
    ));
    self
  }

  /// Exposes an internal node's output port as an external output port.
  ///
  /// This allows internal streams to flow out of the graph through specific internal nodes.
  ///
  /// # Arguments
  ///
  /// * `internal_node` - The name of the internal node
  /// * `internal_port` - The name of the output port on the internal node
  /// * `external_name` - The external port name ("output" or "error")
  ///
  /// # Returns
  ///
  /// The builder instance for method chaining.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::GraphBuilder;
  ///
  /// let builder = GraphBuilder::new("service")
  ///     .expose_output_port("handler", "result", "output")
  ///     .expose_output_port("handler", "errors", "error");
  /// ```
  pub fn expose_output_port(
    mut self,
    internal_node: impl Into<String>,
    internal_port: impl Into<String>,
    external_name: impl Into<String>,
  ) -> Self {
    self.output_mappings.push((
      internal_node.into(),
      internal_port.into(),
      external_name.into(),
    ));
    self
  }

  /// Connects an external input stream to an exposed input port.
  ///
  /// This allows external data to be fed into the graph during execution.
  /// The connection will be established when the graph is built.
  ///
  /// # Arguments
  ///
  /// * `external_port` - The name of the exposed input port
  /// * `input_stream` - The external input stream to connect
  ///
  /// # Returns
  ///
  /// The `GraphBuilder` instance for method chaining.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use tokio::sync::mpsc;
  /// use tokio_stream::wrappers::ReceiverStream;
  ///
  /// let (tx, rx) = mpsc::channel(10);
  /// let input_stream = Box::pin(ReceiverStream::new(rx)) as streamweave::node::InputStream;
  ///
  /// let graph = GraphBuilder::new("my_graph")
  ///     .connect_external_input("configuration", input_stream)
  ///     .build()?;
  /// ```
  pub fn connect_input_channel(
    mut self,
    external_port: impl Into<String>,
    receiver: tokio::sync::mpsc::Receiver<Arc<dyn Any + Send + Sync>>,
  ) -> Self {
    self
      .external_input_channels
      .insert(external_port.into(), receiver);
    self
  }

  /// Connects an external output sender to an exposed output port.
  ///
  /// This allows graph output to be sent to external channels during execution.
  /// The connection will be established when the graph is built.
  ///
  /// # Arguments
  ///
  /// * `external_port` - The name of the exposed output port
  /// * `output_sender` - The external sender to connect
  ///
  /// # Returns
  ///
  /// The `GraphBuilder` instance for method chaining.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use tokio::sync::mpsc;
  /// use std::sync::Arc;
  /// use std::any::Any;
  ///
  /// let (tx, rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  ///
  /// let graph = GraphBuilder::new("my_graph")
  ///     .connect_external_output("output", tx)
  ///     .build()?;
  /// ```
  pub fn connect_output_channel(
    mut self,
    external_port: impl Into<String>,
    sender: tokio::sync::mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  ) -> Self {
    self
      .external_output_channels
      .insert(external_port.into(), sender);
    self
  }

  /// Builds the final Graph instance.
  ///
  /// This method validates all the configuration and creates the Graph.
  /// It will return an error if:
  /// - Node names are not unique
  /// - Edges reference non-existent nodes or ports
  /// - Port mappings are invalid
  ///
  /// # Returns
  ///
  /// A `Result` containing the built `Graph` or an error message.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::GraphBuilder;
  ///
  /// let graph = GraphBuilder::new("my_graph")
  ///     // ... add nodes, connections, and port mappings ...
  ///     .build()
  ///     .expect("Failed to build graph");
  /// ```
  pub fn build(self) -> Result<Graph, String> {
    self.build_with_external_connections()
  }

  /// Builds the graph and returns external connection maps for manual connection.
  ///
  /// This method returns the graph along with the external input/output connection maps
  /// that were configured during building. This allows manual connection of external
  /// channels after graph construction.
  ///
  /// # Returns
  ///
  /// A `Result` containing a tuple of (Graph, external_inputs, external_outputs) or an error.
  pub fn build_with_external_connections(self) -> Result<Graph, String> {
    let mut graph = Graph::new(self.name);

    // Add all nodes
    for (name, node) in self.nodes {
      graph.add_node(name, node)?;
    }

    // Add all edges
    for edge in self.edges {
      graph.add_edge(edge)?;
    }

    // Configure input port mappings
    for (internal_node, internal_port, external_name) in self.input_mappings {
      graph.expose_input_port(&internal_node, &internal_port, &external_name)?;
    }

    // Configure output port mappings
    for (internal_node, internal_port, external_name) in self.output_mappings {
      graph.expose_output_port(&internal_node, &internal_port, &external_name)?;
    }

    // Connect external input channels
    for (external_port, receiver) in self.external_input_channels {
      graph.connect_input_channel(&external_port, receiver)?;
    }

    // Connect external output channels
    for (external_port, sender) in self.external_output_channels {
      graph.connect_output_channel(&external_port, sender)?;
    }

    Ok(graph)
  }
}
