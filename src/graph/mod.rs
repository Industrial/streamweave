//! # Graph API
//!
//! This module provides a graph-based API for StreamWeave that supports
//! Flow-Based Programming (FBP) patterns while maintaining type safety,
//! compile-time validation, and a fluent API.
//!
//! The graph API coexists with the existing linear pipeline API, allowing
//! users to choose the appropriate model for their use case.

mod connection;
mod graph;
mod node;
mod port;
mod router;
mod routers;
mod traits;

// Modules to be implemented in later tasks:
// (none currently)

pub use connection::{CompatibleWith, Connection, ConnectionError, HasInputPort, HasOutputPort};
pub use graph::{
  ConnectionInfo, Graph, GraphBuilder, GraphError, RuntimeGraphBuilder,
};
pub use node::{ConsumerNode, ProducerNode, TransformerNode};
pub use port::{GetPort, PortList, SinglePort};
pub use router::{InputRouter, OutputRouter, RouterError};
pub use routers::{BroadcastRouter, KeyBasedRouter, MergeRouter, RoundRobinRouter};
pub use traits::{NodeKind, NodeTrait};

