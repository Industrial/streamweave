//! Placeholder node for blueprint-derived graphs when no node registry is supplied.
//!
//! A placeholder implements [`Node`](crate::node::Node) with configurable input/output port names
//! but does no real work: [`execute`](crate::node::Node::execute) returns empty streams for each
//! output port. Used by [`blueprint_to_graph`](crate::mermaid::blueprint_to_graph) when building
//! a graph from a parsed blueprint without a registry.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use async_trait::async_trait;
use futures::stream;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// A stub node that exposes given input/output port names but produces empty output streams.
///
/// Used when turning a blueprint into a [`Graph`](crate::graph::Graph) without a node registry:
/// the graph structure is preserved for validation and roundtrip, but execution is a no-op.
#[derive(Debug, Clone)]
pub struct PlaceholderNode {
    /// Node name (blueprint node id).
    name: String,
    /// Input port names (from edges targeting this node).
    input_ports: Vec<String>,
    /// Output port names (from edges originating from this node).
    output_ports: Vec<String>,
}

impl PlaceholderNode {
    /// Creates a placeholder with the given name and port names.
    #[must_use]
    pub fn new(name: String, input_ports: Vec<String>, output_ports: Vec<String>) -> Self {
        Self {
            name,
            input_ports,
            output_ports,
        }
    }
}

#[async_trait]
impl Node for PlaceholderNode {
    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    fn input_port_names(&self) -> &[String] {
        &self.input_ports
    }

    fn output_port_names(&self) -> &[String] {
        &self.output_ports
    }

    fn has_input_port(&self, name: &str) -> bool {
        self.input_ports.iter().any(|p| p == name)
    }

    fn has_output_port(&self, name: &str) -> bool {
        self.output_ports.iter().any(|p| p == name)
    }

    fn execute(
        &self,
        _inputs: InputStreams,
    ) -> Pin<Box<dyn Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>> {
        let output_ports = self.output_ports.clone();
        Box::pin(async move {
            let mut outputs = HashMap::new();
            for port in output_ports {
                let empty =
                    Box::pin(stream::empty::<Arc<dyn std::any::Any + Send + Sync>>()) as crate::node::OutputStream;
                outputs.insert(port, empty);
            }
            Ok(outputs)
        })
    }
}
