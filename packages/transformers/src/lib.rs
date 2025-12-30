#![doc = include_str!("../README.md")]

// Declare all transformer modules
// Transformers are now in flat structure: src/{transformer_name}_transformer.rs
pub mod batch_transformer;
pub mod circuit_breaker_transformer;
pub mod filter_transformer;
pub mod group_by_transformer;
pub mod interleave_transformer;
pub mod limit_transformer;
pub mod map_transformer;
pub mod merge_transformer;
pub mod message_dedupe_transformer;
pub mod moving_average_transformer;
pub mod ordered_merge_transformer;
pub mod partition_transformer;
pub mod rate_limit_transformer;
pub mod reduce_transformer;
pub mod retry_transformer;
pub mod round_robin_transformer;
pub mod router_transformer;
pub mod running_sum_transformer;
pub mod sample_transformer;
pub mod skip_transformer;
pub mod sort_transformer;
pub mod split_at_transformer;
pub mod split_transformer;
pub mod take_transformer;
pub mod timeout_transformer;
pub mod zip_transformer;

// Re-export transformers for convenience
// Note: Access transformers via their module paths, e.g.:
//   use streamweave_transformers::map_transformer::MapTransformer;
//   use streamweave_transformers::batch_transformer::BatchTransformer;
pub use batch_transformer::*;
pub use circuit_breaker_transformer::*;
pub use filter_transformer::*;
pub use group_by_transformer::*;
pub use interleave_transformer::*;
pub use limit_transformer::*;
pub use map_transformer::*;
pub use merge_transformer::*;
pub use message_dedupe_transformer::*;
pub use moving_average_transformer::*;
pub use ordered_merge_transformer::*;
pub use partition_transformer::*;
pub use rate_limit_transformer::*;
pub use reduce_transformer::*;
pub use retry_transformer::*;
pub use round_robin_transformer::*;
pub use router_transformer::*;
pub use running_sum_transformer::*;
pub use sample_transformer::*;
pub use skip_transformer::*;
pub use sort_transformer::*;
pub use split_at_transformer::*;
pub use split_transformer::*;
pub use take_transformer::*;
pub use timeout_transformer::*;
pub use zip_transformer::*;
