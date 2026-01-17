//! # Graph-Based Execution for StreamWeave
//!
//! This module provides comprehensive graph-based execution capabilities for StreamWeave,
//! enabling producers, transformers, and consumers to be connected in complex graph
//! topologies for sophisticated data processing pipelines.
//!
//! ## Overview
//!
//! The graph module provides:
//!
//! - **Graph Construction**: Type-safe graph builders with compile-time validation
//! - **Complex Topologies**: Support for fan-in, fan-out, and complex routing patterns
//! - **Node System**: Wrapper nodes for producers, transformers, and consumers
//! - **Execution Engine**: Concurrent execution of graph nodes with stream routing
//! - **Zero-Copy Support**: Efficient zero-copy data sharing for in-process execution
//! - **Distributed Execution**: Support for distributed graph execution across nodes
//! - **Serialization**: Message serialization for distributed execution
//! - **Routing**: Flexible routing strategies (round-robin, broadcast, custom)
//!
//! ## Core Components
//!
//! - **Graph**: The main graph structure for executing data processing pipelines
//! - **GraphBuilder**: Type-safe builder for constructing graphs with compile-time validation
//! - **RuntimeGraphBuilder**: Dynamic builder for runtime graph construction
//! - **Nodes**: Wrapper types for producers, transformers, and consumers in graphs
//! - **Execution**: Execution engine for running graphs with concurrent node execution
//! - **Router**: Routing strategies for distributing data across multiple outputs
//! - **Channels**: Communication channels between graph nodes
//!
//! ## Universal Message Model
//!
//! **All data flowing through graphs is automatically wrapped in `Message<T>`.** This ensures
//! message IDs, metadata, and error correlation are preserved throughout the graph execution.
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::{Graph, GraphBuilder};
//! use streamweave::producers::VecProducer;
//! use streamweave::transformers::MapTransformer;
//! use streamweave::consumers::VecConsumer;
//!
//! let graph = GraphBuilder::new()
//!     .add_producer("source", VecProducer::new(vec![1, 2, 3]))
//!     .add_transformer("transform", MapTransformer::new(|x: i32| x * 2))
//!     .add_consumer("sink", VecConsumer::new())
//!     .connect("source", "transform")
//!     .connect("transform", "sink")
//!     .build();
//! ```

#[cfg(test)]
mod edge_test;
#[cfg(test)]
mod graph_test;
#[cfg(test)]
mod node_test;

pub mod edge;
#[allow(clippy::module_inception)]
pub mod graph;
pub mod node;
pub mod nodes;
