//! Mermaid flowchart (`.mmd`) import/export for StreamWeave graphs.
//!
//! See [streamweave-mermaid-convention](../../../docs/streamweave-mermaid-convention.md) for the
//! encoding convention. This module provides types and helpers to parse `.mmd` into a graph
//! blueprint and to export a graph (or blueprint) to `.mmd` source.
//!
//! **Registry vs placeholders:** Without a node registry, [`blueprint_to_graph`](blueprint_to_graph::blueprint_to_graph)
//! builds a graph using [placeholder](placeholder_node::PlaceholderNode) nodes only. That graph is
//! suitable for structure validation and roundtrip (parse → export → parse), but execution produces
//! no data. See the convention doc, section "Registry vs placeholders".

pub mod blueprint;
pub mod blueprint_to_graph;
pub use blueprint_to_graph::NodeRegistry;
pub mod convention;
pub mod export;
pub mod parse;
pub mod placeholder_node;
pub mod sidecar;
