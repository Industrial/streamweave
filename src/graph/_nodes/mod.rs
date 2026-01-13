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
//! use crate::graph::nodes::ProducerNode;
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
//! use crate::graph::nodes::TransformerNode;
//! use crate::transformers::MapTransformer;
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
//! use crate::graph::nodes::ConsumerNode;
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
//! use crate::graph::{GraphBuilder, GraphExecution};
//! use crate::graph::nodes::{ProducerNode, TransformerNode, ConsumerNode};
//! use streamweave_array::ArrayProducer;
//! use crate::transformers::MapTransformer;
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
pub mod array_concat;
pub mod array_contains;
pub mod array_find;
pub mod array_index_of;
pub mod array_join;
pub mod array_length;
pub mod array_modify;
pub mod array_reverse;
pub mod array_slice;
pub mod batch;
pub mod broadcast_router;
pub mod circuit_breaker;
pub mod command_execute;
pub mod csv_parse;
pub mod csv_read;
pub mod csv_stringify;
pub mod csv_write;
pub mod database_operation;
pub mod database_query;
pub mod db_mysql_query;
pub mod db_mysql_write;
pub mod db_postgres_query;
pub mod db_postgres_write;
pub mod db_sqlite_query;
pub mod db_sqlite_write;
pub mod delay;
pub mod drop;
pub mod error_branch;
pub mod for_each;
pub mod fs_directory_create;
pub mod fs_directory_list;
pub mod fs_file_name;
pub mod fs_file_read;
pub mod fs_file_write;
pub mod fs_normalize_path;
pub mod fs_parent_path;
pub mod group_by;
pub mod http_request;
pub mod if_router;
pub mod interleave;
pub mod join;
pub mod json_parse;
pub mod json_read;
pub mod json_stringify;
pub mod json_write;
pub mod jsonl_read;
pub mod jsonl_write;
pub mod jsonpath;
pub mod kafka_publish;
pub mod key_based_router;
pub mod limit;
pub mod map;
pub mod match_router;
pub mod math_function;
pub mod math_hyperbolic;
pub mod math_logarithmic;
pub mod math_min_max;
pub mod math_operation;
pub mod math_random;
pub mod math_rounding;
pub mod math_trigonometric;
pub mod math_utility;
pub mod merge_router;
pub mod message_dedupe;
pub mod ml_batched_inference;
pub mod ml_inference;
pub mod object_entries;
pub mod object_has_property;
pub mod object_keys;
pub mod object_merge;
pub mod object_property;
pub mod object_random_member;
pub mod object_values;
pub mod ordered_merge;
pub mod parquet_read;
pub mod parquet_write;
pub mod partition;
pub mod process;
pub mod process_execute;
pub mod rate_limit;
pub mod redis_write;
pub mod reduce;
pub mod repeat;
pub mod retry;
pub mod round_robin_router;
pub mod router;
pub mod sample;
pub mod skip;
pub mod sort;
pub mod split;
pub mod split_at;
pub mod string_case;
pub mod string_char;
pub mod string_concat;
pub mod string_index_of;
pub mod string_join;
pub mod string_length;
pub mod string_match;
pub mod string_pad;
pub mod string_predicate;
pub mod string_repeat;
pub mod string_replace;
pub mod string_reverse;
pub mod string_search;
pub mod string_slice;
pub mod string_split;
pub mod string_split_lines;
pub mod string_split_words;
pub mod string_trim;
pub mod synchronize;
pub mod take;
pub mod tcp_receive;
pub mod tcp_request;
pub mod tcp_send;
pub mod time_window;
pub mod timeout;
pub mod tokio_channel_send;
pub mod trace;
pub mod variables;
pub mod while_loop;
pub mod window;
pub mod wrap;
pub mod xml_parse;
pub mod xml_stringify;
pub mod xor;
pub mod zip;

pub use aggregate::{
  Aggregate, Aggregator, CountAggregator, MaxAggregator, MinAggregator, SumAggregator,
};
pub use array_concat::ArrayConcat;
pub use array_contains::ArrayContains;
pub use array_find::ArrayFind;
// pub use array_index_of::ArrayIndexOf; // TODO: ArrayIndexOf struct not yet implemented
pub use array_join::ArrayJoin;
// pub use array_length::ArrayLength; // TODO: ArrayLength struct not yet implemented, file only contains tests
pub use array_modify::ArrayModify;
pub use array_reverse::ArrayReverse;
pub use array_slice::ArraySlice;
// pub use batch::Batch; // TODO: Batch struct not yet implemented
// pub use broadcast_router::*; // TODO: broadcast_router only contains tests, no exports
pub use circuit_breaker::CircuitBreaker;
pub use command_execute::CommandExecute;
pub use csv_parse::CsvParse;
pub use csv_read::CsvRead;
pub use process_execute::ProcessExecute;
pub mod filter;
pub use crate::transformers::MergeStrategy;
pub use csv_stringify::CsvStringify;
pub use csv_write::CsvWrite;
// pub use database_operation::DatabaseOperationNode; // TODO: DatabaseOperationNode struct not yet implemented, file only contains CsvWrite and tests
pub use database_query::DatabaseQuery;
pub use db_mysql_query::DbMysqlQuery;
pub use db_mysql_write::DbMysqlWrite;
pub use db_postgres_query::DbPostgresQuery;
pub use db_postgres_write::DbPostgresWrite;
pub use db_sqlite_query::DbSqliteQuery;
pub use db_sqlite_write::DbSqliteWrite;
pub use delay::Delay;
pub use drop::Drop;
pub use error_branch::ErrorBranch;
pub use filter::Filter;
pub use for_each::ForEach;
pub use fs_directory_create::FsDirectoryCreate;
pub use fs_directory_list::FsDirectoryList;
pub use fs_file_name::FsFileName;
pub use fs_file_read::FsFileRead;
pub use fs_file_write::FsFileWrite;
pub use fs_normalize_path::FsNormalizePath;
pub use fs_parent_path::FsParentPath;
pub use group_by::GroupBy;
pub use http_request::HttpRequest;
pub use if_router::If;
pub use interleave::Interleave;
pub use join::{Join, JoinStrategy};
pub use json_parse::JsonParse;
pub use json_read::JsonRead;
pub use json_stringify::JsonStringify;
pub use json_write::JsonWrite;
pub use jsonl_read::JsonlRead;
pub use jsonl_write::JsonlWrite;
pub use jsonpath::JsonPath;
pub use kafka_publish::KafkaPublish;
pub use key_based_router::*;
pub use limit::Limit;
pub use map::Map;
pub use match_router::{Match, Pattern, PredicatePattern, RangePattern};
pub use math_function::MathFunctionNode;
pub use math_hyperbolic::MathHyperbolicNode;
pub use math_logarithmic::MathLogarithmicNode;
pub use math_min_max::MathMinMaxNode;
pub use math_operation::MathOperationNode;
pub use math_random::MathRandomNode;
pub use math_rounding::MathRoundingNode;
pub use math_trigonometric::MathTrigonometricNode;
pub use math_utility::MathUtilityNode;
pub use merge_router::*;
pub use message_dedupe::MessageDedupe;
pub use ml_batched_inference::MlBatchedInference;
pub use ml_inference::MlInference;
pub use node::{
  ConsumerNode, ProducerNode, TransformerNode, ValidateConsumerPorts, ValidateProducerPorts,
  ValidateTransformerPorts,
};
pub use object_entries::ObjectEntries;
pub use object_has_property::ObjectHasProperty;
pub use object_keys::ObjectKeys;
pub use object_merge::ObjectMerge;
pub use object_property::ObjectProperty;
pub use object_random_member::ObjectRandomMember;
pub use object_values::ObjectValues;
pub use ordered_merge::OrderedMerge;
pub use parquet_read::ParquetRead;
pub use parquet_write::ParquetWrite;
pub use partition::Partition;
pub use process::Process;
pub use rate_limit::RateLimit;
pub use redis_write::RedisWrite;
pub use reduce::Reduce;
pub use repeat::Repeat;
pub use retry::Retry;
pub use round_robin_router::*;
pub use router::Router;
pub use sample::Sample;
pub use skip::Skip;
pub use sort::Sort;
pub use split::Split;
pub use split_at::SplitAt;
pub use string_case::StringCase;
pub use string_char::StringChar;
pub use string_concat::StringConcat;
pub use string_index_of::StringIndexOf;
pub use string_join::StringJoin;
pub use string_length::StringLength;
pub use string_match::StringMatch;
pub use string_pad::StringPad;
pub use string_predicate::StringPredicate;
pub use string_repeat::StringRepeat;
pub use string_replace::StringReplace;
pub use string_reverse::StringReverse;
pub use string_search::StringSearch;
pub use string_slice::StringSlice;
pub use string_split::StringSplit;
pub use string_split_lines::StringSplitLines;
pub use string_split_words::StringSplitWords;
pub use string_trim::StringTrim;
pub use synchronize::Synchronize;
pub use take::Take;
pub use tcp_receive::TcpReceive;
pub use tcp_request::TcpRequest;
pub use tcp_send::TcpSend;
pub use time_window::TimeWindow;
pub use timeout::{Timeout, TimeoutError};
pub use tokio_channel_send::TokioChannelSend;
pub use trace::Trace;
pub use variables::{GraphVariables, ReadVariable, WriteVariable};
pub use while_loop::While;
pub use window::Window;
pub use wrap::Wrap;
pub use xml_parse::XmlParse;
pub use xml_stringify::XmlStringify;
pub use xor::Xor;
pub use zip::Zip;
