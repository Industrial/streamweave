//! # StreamWeave
//!
//! Composable, async, stream-first computation in pure Rust.
//!
//! StreamWeave provides a powerful framework for building data processing pipelines
//! using a flow-based programming model. It enables you to create complex data flows
//! by connecting nodes together in a graph structure.
//!
//! ## Key Features
//!
//! - **Flow-Based Programming**: Build pipelines by connecting nodes in a graph
//! - **Async-First**: Built on Tokio for high-performance async operations
//! - **Type-Safe**: Leverages Rust's type system for safety
//! - **Composable**: Mix and match nodes to create complex data flows
//! - **Zero-Copy**: Efficient memory management with Arc-based sharing
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use streamweave::graph::Graph;
//! use streamweave::nodes::string::StringSliceNode;
//!
//! // Create a graph and add nodes
//! let mut graph = Graph::new("my_graph".to_string());
//! graph.add_node("slice".to_string(), Box::new(StringSliceNode::new("slice".to_string())))?;
//! ```
//!
//! See the [README](../README.md) for detailed documentation and examples.

// Documentation enforcement - treat missing docs as errors
#![deny(missing_docs)]

/// Graph and node system for building data processing pipelines.
pub mod edge;
/// Graph execution and management.
pub mod graph;
/// Graph builder for constructing graphs with a fluent API.
pub mod graph_builder;
/// Macros for declarative graph construction.
pub mod graph_macros;
/// Logical timestamps for dataflow progress and ordering.
pub mod time;
/// Prometheus-compatible metrics for production observability.
pub mod metrics;
/// Partitioning contract for sharded execution (cluster sharding).
pub mod partitioning;
/// Local checkpoint storage for state recovery.
pub mod checkpoint;
/// Exactly-once state contract for stateful nodes.
pub mod state;
/// Memory pool for efficient allocation.
pub mod memory_pool;
/// Supervision policies for node failure handling.
pub mod supervision;
/// Rebalance protocol for cluster sharding (add/remove workers, state migration).
pub mod rebalance;
/// Built-in auto-scaler: policy config and (when implemented) loop that drives rebalance from metrics.
pub mod scaler;
/// Built-in distribution layer: run N graph instances, route by partition key, merge output.
pub mod distribution;
/// Cluster health aggregation (all shards running, quorum).
pub mod cluster_health;
/// Types for incremental and time-range recomputation.
pub mod incremental;
/// Core node trait and interfaces.
pub mod node;
/// Collection of built-in nodes for common operations.
pub mod nodes;

#[cfg(test)]
mod graph_macros_test;
#[cfg(test)]
mod graph_test;
#[cfg(test)]
mod rebalance_test;
#[cfg(test)]
mod cluster_health_test;
#[cfg(test)]
mod incremental_test;
