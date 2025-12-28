//! Transformers for StreamWeave
//!
//! This package provides a collection of transformers for use in StreamWeave pipelines and graphs.

// Declare all transformer modules
// Note: Directory names use hyphens, but Rust modules use underscores
#[path = "batch/mod.rs"]
pub mod batch;
#[path = "circuit-breaker/mod.rs"]
pub mod circuit_breaker;
#[path = "filter/mod.rs"]
pub mod filter;
#[path = "group-by/mod.rs"]
pub mod group_by;
#[path = "interleave/mod.rs"]
pub mod interleave;
#[path = "limit/mod.rs"]
pub mod limit;
#[path = "map/mod.rs"]
pub mod map;
#[path = "merge/mod.rs"]
pub mod merge;
#[path = "message-dedupe/mod.rs"]
pub mod message_dedupe;
#[cfg(feature = "ml")]
#[path = "ml/mod.rs"]
pub mod ml;
#[path = "moving-average/mod.rs"]
pub mod moving_average;
#[path = "ordered-merge/mod.rs"]
pub mod ordered_merge;
#[path = "partition/mod.rs"]
pub mod partition;
#[path = "rate-limit/mod.rs"]
pub mod rate_limit;
#[path = "reduce/mod.rs"]
pub mod reduce;
#[path = "retry/mod.rs"]
pub mod retry;
#[path = "round-robin/mod.rs"]
pub mod round_robin;
#[path = "router/mod.rs"]
pub mod router;
#[path = "running-sum/mod.rs"]
pub mod running_sum;
#[path = "sample/mod.rs"]
pub mod sample;
#[path = "skip/mod.rs"]
pub mod skip;
#[path = "sort/mod.rs"]
pub mod sort;
#[path = "split/mod.rs"]
pub mod split;
#[path = "split-at/mod.rs"]
pub mod split_at;
#[path = "take/mod.rs"]
pub mod take;
#[path = "timeout/mod.rs"]
pub mod timeout;
#[path = "zip/mod.rs"]
pub mod zip;

// Note: We don't glob re-export to avoid naming conflicts (many transformers export
// modules named `input`, `output`, `transformer`, etc.). Access transformers via
// their module paths, e.g.:
//   use streamweave_transformers::map::MapTransformer;
//   use streamweave_transformers::running_sum::RunningSumTransformer;
