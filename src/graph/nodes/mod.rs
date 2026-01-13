//! # Graph Node Library
//!
//! This module provides a library of reusable nodes for building streaming graphs.
//! All nodes implement the unified `Node` trait and support zero-copy data passing
//! via `Arc<dyn Any + Send + Sync>`.
//!
//! ## Node Categories
//!
//! - **Source Nodes**: Generate data (0 inputs, 1+ outputs)
//! - **Transform Nodes**: Process data (1+ inputs, 1+ outputs)
//! - **Sink Nodes**: Consume data (1+ inputs, 0 outputs)
//! - **Router Nodes**: Route data (1+ inputs, 1+ outputs with routing logic)

pub mod boolean_logic;
pub mod common;
pub mod condition_node;
pub mod filter_node;
pub mod for_each_node;
pub mod map_node;
pub mod match_node;
pub mod while_loop_node;

#[cfg(test)]
mod condition_node_test;
#[cfg(test)]
mod filter_node_test;
#[cfg(test)]
mod for_each_node_test;
#[cfg(test)]
mod map_node_test;
#[cfg(test)]
mod match_node_test;
#[cfg(test)]
mod while_loop_node_test;

pub use boolean_logic::{AndNode, NandNode, NorNode, NotNode, OrNode, XorNode};
pub use common::BaseNode;
pub use condition_node::{ConditionConfig, ConditionFunction, ConditionNode, condition_config};
pub use filter_node::{FilterConfig, FilterFunction, FilterNode, filter_config};
pub use for_each_node::{ForEachConfig, ForEachFunction, ForEachNode, for_each_config};
pub use map_node::{MapConfig, MapFunction, MapNode, map_config};
pub use match_node::{
  MatchConfig, MatchFunction, MatchNode, match_config, match_exact_string, match_regex,
};
pub use while_loop_node::{
  WhileLoopConditionFunction, WhileLoopConfig, WhileLoopNode, while_loop_config,
};
