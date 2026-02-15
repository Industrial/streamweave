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

pub mod advanced;
pub mod aggregation;
pub mod arithmetic;
pub mod array;
pub mod boolean_logic;
pub mod comparison;
pub mod math;
pub mod object;
pub mod reduction;
pub mod stream;
pub mod string;
pub mod time;
pub mod type_ops;

pub mod bounded_iteration_node;
pub mod common;
pub mod condition_node;
pub mod differential_join_node;
pub mod error_branch_node;
pub mod filter_node;
pub mod for_each_node;
pub mod join_node;
pub mod map_node;
pub mod match_node;
pub mod memoizing_map_node;
pub mod range_node;
pub mod read_variable_node;
pub mod sync_node;
pub mod variable_node;
pub mod while_loop_node;
pub mod write_variable_node;

#[cfg(test)]
mod bounded_iteration_node_test;
#[cfg(test)]
mod condition_node_test;
#[cfg(test)]
mod differential_join_node_test;
#[cfg(test)]
mod error_branch_node_test;
#[cfg(test)]
mod filter_node_test;
#[cfg(test)]
mod for_each_node_test;
#[cfg(test)]
mod join_node_test;
#[cfg(test)]
mod map_node_test;
#[cfg(test)]
mod match_node_test;
#[cfg(test)]
mod memoizing_map_node_test;
#[cfg(test)]
mod range_node_test;
#[cfg(test)]
mod read_variable_node_test;
#[cfg(test)]
mod sync_node_test;
#[cfg(test)]
mod variable_node_test;
#[cfg(test)]
mod while_loop_node_test;
#[cfg(test)]
mod write_variable_node_test;
