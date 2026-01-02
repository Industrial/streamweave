//! # Control Flow Constructs
//!
//! This module provides control flow constructs for Flow-Based Programming patterns
//! in StreamWeave graphs. These constructs enable conditional routing, loops, pattern
//! matching, and other control flow operations while maintaining zero-copy semantics
//! where possible.
//!
//! ## Zero-Copy Strategy
//!
//! - **Routers (If, Match, ErrorBranch)**: Use `Arc` for fan-out when items need to
//!   go to multiple ports. Move items directly when routing to a single port.
//! - **Transformers (ForEach, While, Delay, etc.)**: Use `Cow` for conditional cloning
//!   and leverage existing zero-copy infrastructure from `zero_copy` module.

pub mod aggregate;
pub mod delay;
pub mod error_branch;
pub mod for_each;
pub mod group_by;
pub mod if_router;
pub mod join;
pub mod match_router;
pub mod synchronize;
pub mod timeout;
pub mod variables;
pub mod while_loop;

// Re-export all public types
pub use aggregate::{
  Aggregate, Aggregator, CountAggregator, MaxAggregator, MinAggregator, SumAggregator,
};
pub use delay::Delay;
pub use error_branch::ErrorBranch;
pub use for_each::ForEach;
pub use group_by::GroupBy;
pub use if_router::If;
pub use join::{Join, JoinStrategy};
pub use match_router::{Match, Pattern, PredicatePattern, RangePattern};
pub use synchronize::Synchronize;
pub use timeout::{Timeout, TimeoutError};
pub use variables::{GraphVariables, ReadVariable, WriteVariable};
pub use while_loop::While;
