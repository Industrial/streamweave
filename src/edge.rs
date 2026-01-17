/// An edge connecting two ports in a graph.
///
/// Edges represent connections between nodes in the graph. They specify
/// which output port of a source node connects to which input port of a
/// target node.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::edge::Edge;
///
/// let edge = Edge {
///     source_node: "node1".to_string(),
///     source_port: "out".to_string(),
///     target_node: "node2".to_string(),
///     target_port: "in".to_string(),
/// };
///
/// // Access edge properties
/// assert_eq!(edge.source_node(), "node1");
/// assert_eq!(edge.target_node(), "node2");
/// ```
pub struct Edge {
  /// The name of the source node.
  pub source_node: String,
  /// The name of the source output port.
  pub source_port: String,
  /// The name of the target node.
  pub target_node: String,
  /// The name of the target input port.
  pub target_port: String,
}

impl Edge {
  /// Returns the source node name.
  ///
  /// # Returns
  ///
  /// The name of the source node for this edge.
  pub fn source_node(&self) -> &str {
    &self.source_node
  }

  /// Returns the source port name.
  ///
  /// # Returns
  ///
  /// The name of the output port on the source node.
  pub fn source_port(&self) -> &str {
    &self.source_port
  }

  /// Returns the target node name.
  ///
  /// # Returns
  ///
  /// The name of the target node for this edge.
  pub fn target_node(&self) -> &str {
    &self.target_node
  }

  /// Returns the target port name.
  ///
  /// # Returns
  ///
  /// The name of the input port on the target node.
  pub fn target_port(&self) -> &str {
    &self.target_port
  }
}
