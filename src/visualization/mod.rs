//! # Visualization Module
//!
//! This module provides tools for visualizing StreamWeave pipelines as directed acyclic graphs (DAGs).
//! It supports exporting pipeline representations in multiple formats for visualization and debugging.
//!
//! ## Core Concepts
//!
//! - **DAG Representation**: A graph structure representing the pipeline with nodes (components) and edges (data flow)
//! - **Node Metadata**: Information about each component including type, configuration, and properties
//! - **Format Export**: Export DAGs to common formats like DOT (Graphviz) and JSON
//!
//! ## Example
//!
//! ```rust
//! use streamweave::prelude::*;
//! use streamweave::visualization::{PipelineDag, DagExporter};
//!
//! let pipeline = Pipeline::new()
//!     .with_producer(ArrayProducer::new(vec![1, 2, 3]))
//!     .with_transformer(MapTransformer::new(|x| x * 2))
//!     .with_consumer(VecConsumer::new());
//!
//! // Generate DAG representation
//! let dag = PipelineDag::from_pipeline(&pipeline);
//!
//! // Export to DOT format
//! let dot = dag.to_dot();
//! println!("{}", dot);
//!
//! // Export to JSON
//! let json = dag.to_json().unwrap();
//! println!("{}", json);
//! ```

pub mod dag;
pub mod debug;
pub mod exporter;
pub mod realtime;
pub mod ui;

#[cfg(not(target_arch = "wasm32"))]
pub mod server;

pub use dag::{DagEdge, DagNode, NodeKind, NodeMetadata, PipelineDag};
pub use debug::{Breakpoint, DataSnapshot, DebugState, Debugger};
pub use exporter::DagExporter;
pub use realtime::{NodeMetrics, NodeMetricsSnapshot, PipelineMetrics};
pub use ui::generate_standalone_html;

#[cfg(not(target_arch = "wasm32"))]
pub use server::VisualizationServer;
