//! Transformers module.
//!
//! This module provides a collection of stream transformers for processing and
//! transforming data as it flows through pipelines.

/// Backpressure transformer for managing stream flow control.
pub mod backpressure;
/// Batch transformer for grouping items into batches.
pub mod batch;
/// Broadcast transformer for sending items to multiple outputs.
pub mod broadcast;
/// Buffer transformer for buffering stream items.
pub mod buffer;
/// Chunk transformer for splitting streams into chunks.
pub mod chunk;
/// Concat transformer for concatenating streams.
pub mod concat;
/// Dedupe transformer for removing duplicate items.
pub mod dedupe;
/// Distinct transformer for keeping only distinct items.
pub mod distinct;
/// Filter transformer for filtering stream items.
pub mod filter;
/// Flat map transformer for flattening mapped collections.
pub mod flat_map;
/// Flatten transformer for flattening collections.
pub mod flatten;
/// Group by transformer for grouping items by key.
pub mod group_by;
/// Interleave transformer for interleaving multiple streams.
pub mod interleave;
/// Limit transformer for limiting the number of items.
pub mod limit;
/// Map transformer for transforming items.
pub mod map;
/// Merge transformer for merging multiple streams.
pub mod merge;
/// Message dedupe transformer for removing duplicate messages.
pub mod message_dedupe;
/// Moving average transformer for calculating moving averages.
pub mod moving_average;
/// Partition transformer for partitioning streams.
pub mod partition;
/// Reduce transformer for reducing streams to a single value.
pub mod reduce;
/// Round robin transformer for distributing items in round-robin fashion.
pub mod round_robin;
/// Router transformer for routing items to different outputs.
pub mod router;
/// Running sum transformer for calculating running sums.
pub mod running_sum;
/// Skip transformer for skipping items from the beginning.
pub mod skip;
/// Sort transformer for sorting stream items.
pub mod sort;
/// Split transformer for splitting streams based on a predicate.
pub mod split;
/// Split at transformer for splitting streams at a specific index.
pub mod split_at;
/// Take transformer for taking a specified number of items.
pub mod take;
/// Window transformer for creating sliding windows.
pub mod window;
/// Zip transformer for zipping multiple streams.
pub mod zip;

// Transformers requiring tokio runtime features (available in both native and wasm)
#[cfg(any(feature = "native", feature = "wasm"))]
pub mod circuit_breaker;
#[cfg(any(feature = "native", feature = "wasm"))]
pub mod debounce;
#[cfg(any(feature = "native", feature = "wasm"))]
pub mod delay;
#[cfg(any(feature = "native", feature = "wasm"))]
pub mod join;
#[cfg(any(feature = "native", feature = "wasm"))]
pub mod ordered_merge;
#[cfg(any(feature = "native", feature = "wasm"))]
pub mod rate_limit;
#[cfg(any(feature = "native", feature = "wasm"))]
pub mod retry;
#[cfg(any(feature = "native", feature = "wasm"))]
pub mod timeout;

// Transformers requiring random number generation
#[cfg(feature = "random")]
pub mod sample;

// Machine learning transformers (native only - requires ML frameworks)
#[cfg(feature = "ml")]
pub mod ml;
