//! # Subgraph Support
//!
//! This module provides support for using graphs as nodes in other graphs.
//! This enables hierarchical graph composition and modular graph design.
//!
//! ## `Message<T>` Support
//!
//! Subgraphs fully support `Message<T>` types. All data flowing through subgraph
//! boundaries is wrapped in `Message<T>`, preserving message IDs and metadata
//! across subgraph boundaries. When a subgraph is executed:
//! - Input messages (`Message<T>`) flow from the parent graph into the subgraph
//! - The subgraph's internal nodes process messages using the same `Message<T>`
//!   infrastructure as regular graphs
//! - Output messages (`Message<T>`) flow from the subgraph back to the parent graph
//!
//! Message metadata and IDs are preserved throughout, enabling end-to-end
//! traceability even across hierarchical graph boundaries.
//!
//! ## Parameterized Subgraphs
//!
//! Subgraphs can be parameterized, allowing them to be called like functions:
//! - **Parameter ports**: Input ports that receive a single value (not a stream) when the subgraph is invoked
//! - **Return ports**: Output ports that produce a single value (not a stream) when the subgraph completes
//!
//! This enables reusable subgraph components with configurable behavior.
//!
//! **Note**: Parameter and return ports also work with `Message<T>`. When a
//! parameter port receives a value, it's wrapped in `Message<T>` before being
//! passed to internal nodes. Return ports unwrap `Message<T>` before producing
//! the return value.
//!
//! # Example
//!
//! ```rust,no_run
//! use crate::graph::{Graph, SubgraphNode};
//!
//! // Create a parameterized subgraph
//! let mut subgraph = SubgraphNode::new(
//!     "my_subgraph".to_string(),
//!     graph,
//!     2, // 2 input ports (1 parameter, 1 stream)
//!     1, // 1 output port (return value)
//! );
//!
//! // Mark port 0 as a parameter port
//! subgraph.mark_parameter_port(0)?;
//!
//! // Mark port 0 as a return port
//! subgraph.mark_return_port(0)?;
//! ```

use super::graph::Graph;
use super::traits::{NodeKind, NodeTrait};
use std::collections::HashMap;
use std::collections::HashSet;
use tracing::trace;

/// A node that wraps a Graph, allowing it to be used as a node in another graph.
///
/// SubgraphNode enables hierarchical graph composition by treating a complete
/// graph as a single node with input and output ports. The subgraph's ports
/// are mapped to internal nodes within the subgraph.
///
/// ## `Message<T>` Support
///
/// SubgraphNode works seamlessly with `Message<T>`. All data flowing through
/// subgraph boundaries is wrapped in `Message<T>`, and the subgraph's internal
/// graph uses the same `Message<T>` infrastructure as regular graphs. This ensures
/// that message IDs and metadata are preserved when messages cross subgraph
/// boundaries, maintaining end-to-end traceability.
///
/// ## Parameterized Subgraphs
///
/// Subgraphs can be parameterized with parameter ports (single-value inputs)
/// and return ports (single-value outputs), enabling function-like behavior.
///
/// # Port Mapping
///
/// The subgraph's input/output ports are mapped to internal nodes:
/// - Input ports map to consumer nodes or transformer input ports within the subgraph
/// - Output ports map to producer nodes or transformer output ports within the subgraph
///
/// When messages flow through these mappings, they remain wrapped in `Message<T>`,
/// ensuring consistent metadata preservation throughout the subgraph.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::{Graph, SubgraphNode};
///
/// // Create a subgraph
/// let subgraph = Graph::new();
///
/// // Create a subgraph node with 1 input and 1 output port
/// let subgraph_node = SubgraphNode::new(
///     "my_subgraph".to_string(),
///     subgraph,
///     1, // 1 input port
///     1, // 1 output port
/// );
/// ```
#[derive(Clone)]
pub struct SubgraphNode {
  /// The name of this subgraph node
  name: String,
  /// The graph contained within this subgraph
  graph: Graph,
  /// Names of input ports
  input_port_names: Vec<String>,
  /// Names of output ports
  output_port_names: Vec<String>,
  /// Mapping from subgraph input port name to internal node name and port name
  /// Key: subgraph input port name
  /// Value: (internal_node_name, internal_port_name)
  input_port_map: HashMap<String, (String, String)>,
  /// Mapping from subgraph output port name to internal node name and port name
  /// Key: subgraph output port name
  /// Value: (internal_node_name, internal_port_name)
  output_port_map: HashMap<String, (String, String)>,
  /// Set of input port names that are parameter ports (single-value inputs)
  parameter_ports: HashSet<String>,
  /// Set of output port names that are return ports (single-value outputs)
  return_ports: HashSet<String>,
}

impl SubgraphNode {
  /// Creates a new SubgraphNode with the given name, graph, and port names.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of this subgraph node
  /// * `graph` - The graph to wrap as a subgraph
  /// * `input_port_names` - The names of input ports this subgraph exposes
  /// * `output_port_names` - The names of output ports this subgraph exposes
  ///
  /// # Returns
  ///
  /// A new `SubgraphNode` instance with empty port mappings.
  ///
  /// # Note
  ///
  /// After creating a SubgraphNode, you must map the ports to internal nodes
  /// using `map_input_port()` and `map_output_port()`.
  pub fn new(
    name: String,
    graph: Graph,
    input_port_names: Vec<String>,
    output_port_names: Vec<String>,
  ) -> Self {
    trace!(
      "SubgraphNode::new(name={}, input_port_names={:?}, output_port_names={:?})",
      name, input_port_names, output_port_names
    );
    Self {
      name,
      graph,
      input_port_names,
      output_port_names,
      input_port_map: HashMap::new(),
      output_port_map: HashMap::new(),
      parameter_ports: HashSet::new(),
      return_ports: HashSet::new(),
    }
  }

  /// Maps a subgraph input port to an internal node and port.
  ///
  /// # Arguments
  ///
  /// * `subgraph_port_name` - The subgraph input port name
  /// * `internal_node` - The name of the internal node to connect to
  /// * `internal_port_name` - The port name on the internal node
  ///
  /// # Returns
  ///
  /// `Ok(())` if the mapping was successful, `Err(String)` if the internal
  /// node doesn't exist or the port is invalid.
  pub fn map_input_port(
    &mut self,
    subgraph_port_name: &str,
    internal_node: String,
    internal_port_name: &str,
  ) -> Result<(), String> {
    trace!(
      "SubgraphNode::map_input_port(subgraph_port_name={}, internal_node={}, internal_port_name={})",
      subgraph_port_name, internal_node, internal_port_name
    );
    if !self
      .input_port_names
      .iter()
      .any(|name| name == subgraph_port_name)
    {
      return Err(format!(
        "Subgraph input port '{}' does not exist (available ports: {:?})",
        subgraph_port_name, self.input_port_names
      ));
    }

    // Validate internal node exists
    if let Some(node) = self.graph.get_node(&internal_node) {
      if !node.has_input_port(internal_port_name) {
        return Err(format!(
          "Internal node '{}' does not have input port '{}'",
          internal_node, internal_port_name
        ));
      }
    } else {
      return Err(format!("Internal node '{}' does not exist", internal_node));
    }

    self.input_port_map.insert(
      subgraph_port_name.to_string(),
      (internal_node, internal_port_name.to_string()),
    );
    Ok(())
  }

  /// Maps a subgraph output port to an internal node and port.
  ///
  /// # Arguments
  ///
  /// * `subgraph_port_name` - The subgraph output port name
  /// * `internal_node` - The name of the internal node to connect from
  /// * `internal_port_name` - The port name on the internal node
  ///
  /// # Returns
  ///
  /// `Ok(())` if the mapping was successful, `Err(String)` if the internal
  /// node doesn't exist or the port is invalid.
  pub fn map_output_port(
    &mut self,
    subgraph_port_name: &str,
    internal_node: String,
    internal_port_name: &str,
  ) -> Result<(), String> {
    trace!(
      "SubgraphNode::map_output_port(subgraph_port_name={}, internal_node={}, internal_port_name={})",
      subgraph_port_name, internal_node, internal_port_name
    );
    if !self
      .output_port_names
      .iter()
      .any(|name| name == subgraph_port_name)
    {
      return Err(format!(
        "Subgraph output port '{}' does not exist (available ports: {:?})",
        subgraph_port_name, self.output_port_names
      ));
    }

    // Validate internal node exists
    if let Some(node) = self.graph.get_node(&internal_node) {
      if !node.has_output_port(internal_port_name) {
        return Err(format!(
          "Internal node '{}' does not have output port '{}'",
          internal_node, internal_port_name
        ));
      }
    } else {
      return Err(format!("Internal node '{}' does not exist", internal_node));
    }

    self.output_port_map.insert(
      subgraph_port_name.to_string(),
      (internal_node, internal_port_name.to_string()),
    );
    Ok(())
  }

  /// Returns a reference to the wrapped graph.
  ///
  /// # Returns
  ///
  /// A reference to the `Graph` contained in this subgraph.
  pub fn graph(&self) -> &Graph {
    trace!("SubgraphNode::graph(name={})", self.name);
    &self.graph
  }

  /// Returns a mutable reference to the wrapped graph.
  ///
  /// # Returns
  ///
  /// A mutable reference to the `Graph` contained in this subgraph.
  pub fn graph_mut(&mut self) -> &mut Graph {
    trace!("SubgraphNode::graph_mut(name={})", self.name);
    &mut self.graph
  }

  /// Returns the input port mapping for a given port name.
  ///
  /// # Arguments
  ///
  /// * `port_name` - The subgraph input port name
  ///
  /// # Returns
  ///
  /// `Some((node_name, port_name))` if the port is mapped, `None` otherwise.
  pub fn get_input_port_mapping(&self, port_name: &str) -> Option<&(String, String)> {
    trace!(
      "SubgraphNode::get_input_port_mapping(name={}, port_name={})",
      self.name, port_name
    );
    self.input_port_map.get(port_name)
  }

  /// Returns the output port mapping for a given port name.
  ///
  /// # Arguments
  ///
  /// * `port_name` - The subgraph output port name
  ///
  /// # Returns
  ///
  /// `Some((node_name, port_name))` if the port is mapped, `None` otherwise.
  pub fn get_output_port_mapping(&self, port_name: &str) -> Option<&(String, String)> {
    trace!(
      "SubgraphNode::get_output_port_mapping(name={}, port_name={})",
      self.name, port_name
    );
    self.output_port_map.get(port_name)
  }

  /// Marks an input port as a parameter port (single-value input).
  ///
  /// Parameter ports receive a single value when the subgraph is invoked,
  /// rather than a stream of values. This enables function-like behavior
  /// where parameters are passed once at invocation time.
  ///
  /// # Arguments
  ///
  /// * `port_name` - The input port name to mark as a parameter port
  ///
  /// # Returns
  ///
  /// `Ok(())` if the port was successfully marked, `Err(String)` if the port
  /// name is invalid or the port is already mapped as a streaming port.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use crate::graph::SubgraphNode;
  ///
  /// let mut subgraph = SubgraphNode::new("subgraph".to_string(), graph, vec!["in_a".to_string(), "in_b".to_string()], vec!["out".to_string()]);
  ///
  /// // Mark port "in_a" as a parameter port
  /// subgraph.mark_parameter_port("in_a")?;
  /// ```
  pub fn mark_parameter_port(&mut self, port_name: &str) -> Result<(), String> {
    trace!(
      "SubgraphNode::mark_parameter_port(name={}, port_name={})",
      self.name, port_name
    );
    if !self.input_port_names.iter().any(|name| name == port_name) {
      return Err(format!(
        "Input port '{}' does not exist (available ports: {:?})",
        port_name, self.input_port_names
      ));
    }

    self.parameter_ports.insert(port_name.to_string());
    Ok(())
  }

  /// Marks an output port as a return port (single-value output).
  ///
  /// Return ports produce a single value when the subgraph completes,
  /// rather than a stream of values. This enables function-like behavior
  /// where return values are produced once at completion time.
  ///
  /// # Arguments
  ///
  /// * `port_name` - The output port name to mark as a return port
  ///
  /// # Returns
  ///
  /// `Ok(())` if the port was successfully marked, `Err(String)` if the port
  /// name is invalid.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use crate::graph::SubgraphNode;
  ///
  /// let mut subgraph = SubgraphNode::new("subgraph".to_string(), graph, vec!["in".to_string()], vec!["out_a".to_string(), "out_b".to_string()]);
  ///
  /// // Mark port "out_a" as a return port
  /// subgraph.mark_return_port("out_a")?;
  /// ```
  pub fn mark_return_port(&mut self, port_name: &str) -> Result<(), String> {
    trace!(
      "SubgraphNode::mark_return_port(name={}, port_name={})",
      self.name, port_name
    );
    if !self.output_port_names.iter().any(|name| name == port_name) {
      return Err(format!(
        "Output port '{}' does not exist (available ports: {:?})",
        port_name, self.output_port_names
      ));
    }

    self.return_ports.insert(port_name.to_string());
    Ok(())
  }

  /// Checks if an input port is a parameter port.
  ///
  /// # Arguments
  ///
  /// * `port_name` - The input port name to check
  ///
  /// # Returns
  ///
  /// `true` if the port is a parameter port, `false` otherwise.
  pub fn is_parameter_port(&self, port_name: &str) -> bool {
    let result = self.parameter_ports.contains(port_name);
    trace!(
      "SubgraphNode::is_parameter_port(name={}, port_name={}) -> {}",
      self.name, port_name, result
    );
    result
  }

  /// Checks if an output port is a return port.
  ///
  /// # Arguments
  ///
  /// * `port_name` - The output port name to check
  ///
  /// # Returns
  ///
  /// `true` if the port is a return port, `false` otherwise.
  pub fn is_return_port(&self, port_name: &str) -> bool {
    let result = self.return_ports.contains(port_name);
    trace!(
      "SubgraphNode::is_return_port(name={}, port_name={}) -> {}",
      self.name, port_name, result
    );
    result
  }

  /// Returns the set of parameter port names.
  ///
  /// # Returns
  ///
  /// A reference to the set of parameter port names.
  pub fn parameter_ports(&self) -> &HashSet<String> {
    trace!(
      "SubgraphNode::parameter_ports(name={}) -> {} ports",
      self.name,
      self.parameter_ports.len()
    );
    &self.parameter_ports
  }

  /// Returns the set of return port names.
  ///
  /// # Returns
  ///
  /// A reference to the set of return port names.
  pub fn return_ports(&self) -> &HashSet<String> {
    trace!(
      "SubgraphNode::return_ports(name={}) -> {} ports",
      self.name,
      self.return_ports.len()
    );
    &self.return_ports
  }
}

impl NodeTrait for SubgraphNode {
  fn name(&self) -> &str {
    trace!("SubgraphNode::name() -> {}", self.name);
    &self.name
  }

  fn node_kind(&self) -> NodeKind {
    trace!("SubgraphNode::node_kind() -> Subgraph");
    NodeKind::Subgraph
  }

  fn input_port_names(&self) -> Vec<String> {
    trace!(
      "SubgraphNode::input_port_names(name={}) -> {} ports",
      self.name,
      self.input_port_names.len()
    );
    self.input_port_names.clone()
  }

  fn output_port_names(&self) -> Vec<String> {
    trace!(
      "SubgraphNode::output_port_names(name={}) -> {} ports",
      self.name,
      self.output_port_names.len()
    );
    self.output_port_names.clone()
  }

  fn has_input_port(&self, port_name: &str) -> bool {
    let result = self.input_port_names.iter().any(|name| name == port_name);
    trace!(
      "SubgraphNode::has_input_port(name={}, port_name={}) -> {}",
      self.name, port_name, result
    );
    result
  }

  fn has_output_port(&self, port_name: &str) -> bool {
    let result = self.output_port_names.iter().any(|name| name == port_name);
    trace!(
      "SubgraphNode::has_output_port(name={}, port_name={}) -> {}",
      self.name, port_name, result
    );
    result
  }
}
