//! # Graph API
//!
//! This module provides a graph-based API for StreamWeave that supports
//! Flow-Based Programming (FBP) patterns while maintaining type safety,
//! compile-time validation, and a fluent API.
//!
//! The graph API coexists with the existing linear pipeline API, allowing
//! users to choose the appropriate model for their use case.

pub mod connection;
pub mod execution;
#[allow(clippy::module_inception)]
pub mod graph;
pub mod node;
pub mod port;
pub mod router;
pub mod routers;
pub mod stateful;
pub mod subgraph;
pub mod traits;
pub mod windowing;

// Modules to be implemented in later tasks:
// (none currently)

pub use connection::{CompatibleWith, Connection, ConnectionError, HasInputPort, HasOutputPort};
pub use execution::{ExecutionError, ExecutionState, GraphExecution, GraphExecutor};
pub use graph::{
  AppendNode, ConnectionInfo, ContainsNodeType, Graph, GraphBuilder, GraphError, HasConnections,
  HasNodes, RuntimeGraphBuilder,
};
pub use node::{ConsumerNode, ProducerNode, TransformerNode};
pub use port::{GetPort, PortList, SinglePort};
pub use router::{InputRouter, OutputRouter, RouterError};
pub use routers::{BroadcastRouter, KeyBasedRouter, MergeRouter, RoundRobinRouter};
pub use stateful::{StatefulNode, get_node_state, is_stateful_node};
pub use subgraph::SubgraphNode;
pub use traits::{NodeKind, NodeTrait};
pub use windowing::{
  GraphWindowConfig, WindowSize, WindowType, WindowedNode, create_window_node, is_windowed_node,
};
