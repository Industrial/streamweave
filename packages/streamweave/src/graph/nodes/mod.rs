//! # Graph Node Types
//!
//! This module provides node types for graph-based execution.
//! Nodes wrap Producer, Transformer, and Consumer components
//! to enable them to participate in graph execution.
//!
//! ## `Message<T>` Based Data Flow
//!
//! **All data flowing through graph nodes is automatically wrapped in `Message<T>`.** Nodes handle
//! message wrapping and unwrapping internally, so you work with raw types while the system ensures
//! message IDs and metadata are preserved.
//!
//! ### How Nodes Handle `Message<T>`
//!
//! - **ProducerNode**: Wraps producer output in `Message<T>` before sending to output channels.
//!   The producer itself works with raw types, but the node wraps each item in a message.
//!
//! - **TransformerNode**: Unwraps `Message<T::Input>` from input channels, passes raw types to the
//!   transformer, then wraps `Message<T::Output>` before sending to output channels. Message IDs
//!   and metadata are preserved through transformations.
//!
//! - **ConsumerNode**: Unwraps `Message<C::Input>` from input channels before passing raw types
//!   to the consumer. The consumer receives unwrapped payloads.
//!
//! - **Control Flow Nodes**: Operate on raw types internally, with `Message<T>` wrapping/unwrapping
//!   handled by wrapper nodes. Message IDs and metadata are preserved.
//!
//! - **Router Nodes**: Unwrap `Message<T>` from input channels, route raw types, then wrap
//!   `Message<T>` before sending to output channels. All routed messages preserve their IDs and metadata.
//!
//! ## Core Node Types
//!
//! ### ProducerNode
//!
//! Wraps a `Producer` component to enable it to participate in graph execution.
//!
//! **Message Handling**: Automatically wraps producer output in `Message<T>` before sending.
//!
//! ```rust,no_run
//! use streamweave::graph::nodes::ProducerNode;
//! use streamweave_array::ArrayProducer;
//!
//! // Producer works with raw types (i32)
//! let producer = ProducerNode::from_producer(
//!     "source".to_string(),
//!     ArrayProducer::new([1, 2, 3]),
//! );
//! // Output: Message<i32> (automatically wrapped)
//! ```
//!
//! ### TransformerNode
//!
//! Wraps a `Transformer` component to enable it to participate in graph execution.
//!
//! **Message Handling**: Unwraps `Message<T::Input>`, transforms raw types, wraps `Message<T::Output>`.
//!
//! ```rust,no_run
//! use streamweave::graph::nodes::TransformerNode;
//! use streamweave::transformers::MapTransformer;
//!
//! // Transformer works with raw types (i32 -> i32)
//! let transformer = TransformerNode::from_transformer(
//!     "double".to_string(),
//!     MapTransformer::new(|x: i32| x * 2),
//! );
//! // Input: Message<i32> (unwrapped to i32)
//! // Output: Message<i32> (wrapped from i32)
//! // Message IDs and metadata are preserved
//! ```
//!
//! ### ConsumerNode
//!
//! Wraps a `Consumer` component to enable it to participate in graph execution.
//!
//! **Message Handling**: Unwraps `Message<C::Input>` before passing to consumer.
//!
//! ```rust,no_run
//! use streamweave::graph::nodes::ConsumerNode;
//! use crate::consumers::VecConsumer;
//!
//! // Consumer works with raw types (i32)
//! let consumer = ConsumerNode::from_consumer(
//!     "sink".to_string(),
//!     VecConsumer::<i32>::new(),
//! );
//! // Input: Message<i32> (unwrapped to i32)
//! ```
//!
//! ## Control Flow Nodes
//!
//! Control flow nodes provide advanced flow-based programming constructs. They operate on raw types
//! internally, with `Message<T>` handling delegated to wrapper nodes (`InputRouterNode`/`OutputRouterNode`).
//!
//! - **Aggregate**: Aggregate items using various aggregators (sum, count, average, etc.)
//! - **Delay**: Delay items by a specified duration
//! - **ErrorBranch**: Route items based on error conditions
//! - **ForEach**: Process each item in a collection
//! - **GroupBy**: Group items by a key function
//! - **If**: Conditional routing based on predicates
//! - **Join**: Join multiple input streams on keys
//! - **Match**: Pattern matching and routing
//! - **Synchronize**: Synchronize multiple input streams
//! - **Timeout**: Apply timeouts to operations
//! - **Variables**: Graph-level variable management
//! - **While**: Loop constructs with conditions
//!
//! **Message Handling**: All control flow nodes preserve message IDs and metadata when routing or transforming data.
//!
//! ## Router Nodes
//!
//! Router nodes handle fan-in and fan-out patterns. They automatically handle `Message<T>` wrapping
//! and unwrapping, preserving message IDs and metadata when distributing messages.
//!
//! - **BroadcastRouter**: Broadcasts items to all output ports. Each output receives a copy with the same message ID.
//! - **KeyBasedRouter**: Routes items based on keys extracted from the payload. Message IDs are preserved.
//! - **MergeRouter**: Merges multiple input streams into a single output. Message IDs from all inputs are preserved.
//! - **RoundRobinRouter**: Distributes items in round-robin fashion across output ports. Message IDs are preserved.
//!
//! **Message Handling**: Router nodes unwrap `Message<T>` from input channels, route raw types, then wrap
//! `Message<T>` before sending to output channels. All routed messages preserve their original IDs and metadata.
//!
//! ## Example: Complete Graph with `Message<T>`
//!
//! ```rust,no_run
//! use streamweave::graph::{GraphBuilder, GraphExecution};
//! use streamweave::graph::nodes::{ProducerNode, TransformerNode, ConsumerNode};
//! use streamweave_array::ArrayProducer;
//! use streamweave::transformers::MapTransformer;
//! use crate::consumers::VecConsumer;
//!
//! // Create a graph - all data flows as Message<T>
//! let graph = GraphBuilder::new()
//!     .node(ProducerNode::from_producer(
//!         "source".to_string(),
//!         ArrayProducer::new([1, 2, 3, 4, 5]),
//!     ))?
//!     .node(TransformerNode::from_transformer(
//!         "double".to_string(),
//!         MapTransformer::new(|x: i32| x * 2),
//!     ))?
//!     .node(ConsumerNode::from_consumer(
//!         "sink".to_string(),
//!         VecConsumer::<i32>::new(),
//!     ))?
//!     .connect_by_name("source", "double")?
//!     .connect_by_name("double", "sink")?
//!     .build();
//!
//! // Execute - messages flow: Message<i32> -> unwrap -> transform -> wrap -> Message<i32>
//! let mut executor = graph.executor();
//! executor.start().await?;
//! ```

pub mod aggregate;
pub mod broadcast_router;
pub mod delay;
pub mod error_branch;
pub mod for_each;
pub mod group_by;
pub mod if_router;
pub mod join;
pub mod key_based_router;
pub mod match_router;
pub mod merge_router;
pub mod node;
pub mod round_robin_router;
pub mod synchronize;
pub mod timeout;
pub mod variables;
pub mod while_loop;

pub use aggregate::{
  Aggregate, Aggregator, CountAggregator, MaxAggregator, MinAggregator, SumAggregator,
};
pub use broadcast_router::*;
pub use delay::Delay;
pub use error_branch::ErrorBranch;
pub use for_each::ForEach;
pub use group_by::GroupBy;
pub use if_router::If;
pub use join::{Join, JoinStrategy};
pub use key_based_router::*;
pub use match_router::{Match, Pattern, PredicatePattern, RangePattern};
pub use merge_router::*;
pub use node::{
  ConsumerNode, ProducerNode, TransformerNode, ValidateConsumerPorts, ValidateProducerPorts,
  ValidateTransformerPorts,
};
pub use round_robin_router::*;
pub use synchronize::Synchronize;
pub use timeout::{Timeout, TimeoutError};
pub use variables::{GraphVariables, ReadVariable, WriteVariable};
pub use while_loop::While;
