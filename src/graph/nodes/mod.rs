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
pub mod common;
pub mod comparison;
pub mod condition_node;
pub mod error_branch_node;
pub mod filter_node;
pub mod for_each_node;
pub mod join_node;
pub mod map_node;
pub mod match_node;
pub mod math;
pub mod object;
pub mod range_node;
pub mod read_variable_node;
pub mod reduction;
pub mod stream;
pub mod string;
pub mod sync_node;
pub mod time;
pub mod type_ops;
pub mod variable_node;
pub mod while_loop_node;
pub mod write_variable_node;

#[cfg(test)]
mod condition_node_test;
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

pub use advanced::{BreakNode, RepeatNode};
pub use aggregation::{AverageNode, CountNode, MaxAggregateNode, MinAggregateNode, SumNode};
pub use arithmetic::{AddNode, DivideNode, ModuloNode, MultiplyNode, PowerNode, SubtractNode};
pub use array::{
  ArrayConcatNode, ArrayContainsNode, ArrayFilterNode, ArrayFlattenNode, ArrayIndexNode,
  ArrayIndexOfNode, ArrayJoinNode, ArrayLengthNode, ArrayMapNode, ArrayReverseNode, ArraySliceNode,
  ArraySortNode, ArraySplitNode, ArrayUniqueNode,
};
pub use boolean_logic::{AndNode, NandNode, NorNode, NotNode, OrNode, XorNode};
pub use common::BaseNode;
pub use comparison::{
  EqualNode, GreaterThanNode, GreaterThanOrEqualNode, LessThanNode, LessThanOrEqualNode,
  NotEqualNode,
};
pub use condition_node::{ConditionConfig, ConditionFunction, ConditionNode, condition_config};
pub use error_branch_node::{ErrorBranchConfig, ErrorBranchNode};
pub use filter_node::{FilterConfig, FilterFunction, FilterNode, filter_config};
pub use for_each_node::{ForEachConfig, ForEachFunction, ForEachNode, for_each_config};
pub use join_node::{
  JoinCombineFunction, JoinConfig, JoinKeyFunction, JoinNode, JoinStrategy, join_config,
};
pub use map_node::{MapConfig, MapFunction, MapNode, map_config};
pub use match_node::{
  MatchConfig, MatchFunction, MatchNode, match_config, match_exact_string, match_regex,
};
pub use math::{
  AbsNode, CeilNode, ExpNode, FloorNode, LogNode, MaxNode, MinNode, RoundNode, SqrtNode,
};
pub use object::{
  ObjectDeletePropertyNode, ObjectEntriesNode, ObjectHasPropertyNode, ObjectKeysNode,
  ObjectMergeNode, ObjectPropertyNode, ObjectSetPropertyNode, ObjectValuesNode,
};
pub use range_node::{RangeConfig, RangeNode};
pub use read_variable_node::{ReadVariableConfig, ReadVariableNode};
pub use reduction::{
  AggregateConfig, AggregateConfigWrapper, AggregateNode, AggregatorFunction, GroupByConfig,
  GroupByConfigWrapper, GroupByKeyFunction, GroupByNode, ReduceConfig, ReduceConfigWrapper,
  ReduceFunction, ReduceNode, aggregate_config, group_by_config, reduce_config,
};
pub use stream::{
  DropNode, InterleaveNode, LimitNode, MergeNode, SampleNode, SkipNode, TakeNode, ZipNode,
};
pub use string::{
  StringCaseNode, StringConcatNode, StringContainsNode, StringEndsWithNode, StringEqualNode,
  StringJoinNode, StringLengthNode, StringMatchNode, StringPadNode, StringReplaceNode,
  StringReverseNode, StringSliceNode, StringSplitNode, StringStartsWithNode, StringTrimNode,
};
pub use sync_node::{SyncConfig, SyncNode};
pub use time::{DelayNode, TimeoutNode, TimerNode, TimestampNode};
pub use type_ops::{
  IsArrayNode, IsBooleanNode, IsNullNode, IsNumberNode, IsObjectNode, IsStringNode, ToArrayNode,
  ToBooleanNode, ToNumberNode, ToStringNode, TypeOfNode,
};
pub use variable_node::{VariableConfig, VariableNode};
pub use while_loop_node::{
  WhileLoopConditionFunction, WhileLoopConfig, WhileLoopNode, while_loop_config,
};
pub use write_variable_node::{WriteVariableConfig, WriteVariableNode};
