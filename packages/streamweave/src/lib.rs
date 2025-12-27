//! StreamWeave - Composable, async, stream-first computation in pure Rust
//!
//! This is the meta-package that re-exports all StreamWeave functionality.
//! Use feature flags to include only the packages you need.

// Ambiguous glob re-exports are expected in this meta-package
#![allow(ambiguous_glob_reexports)]

// Core packages
#[cfg(feature = "core")]
pub use streamweave_core::*;

#[cfg(feature = "pipeline")]
pub use streamweave_pipeline::*;

#[cfg(feature = "graph")]
pub use streamweave_graph::*;

#[cfg(feature = "message")]
pub use streamweave_message::*;

#[cfg(feature = "error")]
pub use streamweave_error::*;

#[cfg(feature = "window")]
pub use streamweave_window::*;

#[cfg(feature = "stateful")]
pub use streamweave_stateful::*;

#[cfg(feature = "transaction")]
pub use streamweave_transaction::*;

#[cfg(feature = "offset")]
pub use streamweave_offset::*;

#[cfg(feature = "metrics")]
pub use streamweave_metrics::*;

#[cfg(feature = "visualization")]
pub use streamweave_visualization::*;

// Producer packages
#[cfg(feature = "producer-array")]
pub use streamweave_producer_array::*;

#[cfg(feature = "producer-vec")]
pub use streamweave_producer_vec::*;

#[cfg(feature = "producer-range")]
pub use streamweave_producer_range::*;

#[cfg(feature = "producer-file")]
pub use streamweave_producer_file::*;

#[cfg(feature = "producer-csv")]
pub use streamweave_producer_csv::*;

#[cfg(feature = "producer-jsonl")]
pub use streamweave_producer_jsonl::*;

#[cfg(feature = "producer-parquet")]
pub use streamweave_producer_parquet::*;

#[cfg(feature = "producer-kafka")]
pub use streamweave_producer_kafka::*;

#[cfg(feature = "producer-redis-streams")]
pub use streamweave_producer_redis_streams::*;

#[cfg(feature = "producer-database")]
pub use streamweave_producer_database::*;

#[cfg(feature = "producer-http-poll")]
pub use streamweave_producer_http_poll::*;

#[cfg(feature = "producer-env-var")]
pub use streamweave_producer_env_var::*;

#[cfg(feature = "producer-command")]
pub use streamweave_producer_command::*;

// Consumer packages
#[cfg(feature = "consumer-array")]
pub use streamweave_consumer_array::*;

#[cfg(feature = "consumer-vec")]
pub use streamweave_consumer_vec::*;

#[cfg(feature = "consumer-file")]
pub use streamweave_consumer_file::*;

#[cfg(feature = "consumer-csv")]
pub use streamweave_consumer_csv::*;

#[cfg(feature = "consumer-jsonl")]
pub use streamweave_consumer_jsonl::*;

#[cfg(feature = "consumer-parquet")]
pub use streamweave_consumer_parquet::*;

#[cfg(feature = "consumer-kafka")]
pub use streamweave_consumer_kafka::*;

#[cfg(feature = "consumer-redis-streams")]
pub use streamweave_consumer_redis_streams::*;

#[cfg(feature = "consumer-console")]
pub use streamweave_consumer_console::*;

#[cfg(feature = "consumer-channel")]
pub use streamweave_consumer_channel::*;

#[cfg(feature = "consumer-command")]
pub use streamweave_consumer_command::*;

// Transformer packages
#[cfg(feature = "transformer-map")]
pub use streamweave_transformer_map::*;

#[cfg(feature = "transformer-filter")]
pub use streamweave_transformer_filter::*;

#[cfg(feature = "transformer-batch")]
pub use streamweave_transformer_batch::*;

#[cfg(feature = "transformer-reduce")]
pub use streamweave_transformer_reduce::*;

#[cfg(feature = "transformer-merge")]
pub use streamweave_transformer_merge::*;

#[cfg(feature = "transformer-split")]
pub use streamweave_transformer_split::*;

#[cfg(feature = "transformer-retry")]
pub use streamweave_transformer_retry::*;

#[cfg(feature = "transformer-rate-limit")]
pub use streamweave_transformer_rate_limit::*;

#[cfg(feature = "transformer-circuit-breaker")]
pub use streamweave_transformer_circuit_breaker::*;

#[cfg(feature = "transformer-window")]
pub use streamweave_transformer_window::*;

#[cfg(feature = "transformer-group-by")]
pub use streamweave_transformer_group_by::*;

#[cfg(feature = "transformer-sort")]
pub use streamweave_transformer_sort::*;

#[cfg(feature = "transformer-take")]
pub use streamweave_transformer_take::*;

#[cfg(feature = "transformer-skip")]
pub use streamweave_transformer_skip::*;

#[cfg(feature = "transformer-sample")]
pub use streamweave_transformer_sample::*;

#[cfg(feature = "transformer-router")]
pub use streamweave_transformer_router::*;

#[cfg(feature = "transformer-round-robin")]
pub use streamweave_transformer_round_robin::*;

#[cfg(feature = "transformer-ordered-merge")]
pub use streamweave_transformer_ordered_merge::*;

#[cfg(feature = "transformer-message-dedupe")]
pub use streamweave_transformer_message_dedupe::*;

#[cfg(feature = "transformer-interleave")]
pub use streamweave_transformer_interleave::*;

#[cfg(feature = "transformer-partition")]
pub use streamweave_transformer_partition::*;

#[cfg(feature = "transformer-split-at")]
pub use streamweave_transformer_split_at::*;

#[cfg(feature = "transformer-timeout")]
pub use streamweave_transformer_timeout::*;

#[cfg(feature = "transformer-zip")]
pub use streamweave_transformer_zip::*;

#[cfg(feature = "transformer-limit")]
pub use streamweave_transformer_limit::*;

#[cfg(feature = "transformer-running-sum")]
pub use streamweave_transformer_running_sum::*;

#[cfg(feature = "transformer-moving-average")]
pub use streamweave_transformer_moving_average::*;

#[cfg(feature = "transformer-ml")]
pub use streamweave_transformer_ml::*;

// Integration packages
#[cfg(feature = "integration-distributed")]
pub use streamweave_integration_distributed::*;

#[cfg(feature = "integration-http-server")]
pub use streamweave_integration_http_server::*;

#[cfg(feature = "integration-sql")]
pub use streamweave_integration_sql::*;
