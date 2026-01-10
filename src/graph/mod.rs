//! Graph-based execution for StreamWeave.
//!
//! This module provides graph-based execution capabilities, allowing
//! producers, transformers, and consumers to be connected in a graph
//! structure for complex data processing pipelines.

pub mod batching;
pub mod channels;
pub mod compression;
pub mod connection;
pub mod execution;
#[allow(clippy::module_inception)]
pub mod graph;
pub mod graph_builder;
pub mod http_server;
pub mod nodes;
pub mod router;
pub mod serialization;
pub mod shared_memory_channel;
pub mod stateful;
pub mod subgraph;
pub mod tcp_server;
pub mod throughput;
pub mod traits;
pub mod windowing;
pub mod zero_copy;

pub use batching::*;
pub use channels::*;
pub use compression::*;
pub use connection::*;
pub use execution::*;
pub use graph::*;
pub use graph_builder::*;
pub use http_server::*;
pub use nodes::*;
pub use router::*;
pub use serialization::*;
pub use shared_memory_channel::*;
pub use subgraph::*;
pub use tcp_server::*;
pub use throughput::*;
pub use traits::*;
pub use zero_copy::*;
