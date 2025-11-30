//! # Graph API
//!
//! This module provides a graph-based API for StreamWeave that supports
//! Flow-Based Programming (FBP) patterns while maintaining type safety,
//! compile-time validation, and a fluent API.
//!
//! The graph API coexists with the existing linear pipeline API, allowing
//! users to choose the appropriate model for their use case.

mod connection;
mod node;
mod port;

// Modules to be implemented in later tasks:
// mod graph;

pub use connection::{CompatibleWith, Connection, ConnectionError, HasInputPort, HasOutputPort};
pub use node::{ConsumerNode, ProducerNode, TransformerNode};
pub use port::{GetPort, PortList, SinglePort};

// Exports to be added in later tasks:
// pub use graph::Graph;

