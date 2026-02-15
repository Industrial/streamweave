//! Built-in auto-scaler for cluster sharding.
//!
//! Provides configuration and (when implemented) a policy engine and loop that
//! read metrics and call the rebalance coordinator to scale the number of shards.
//! See [auto-scaling-clusters.md](../docs/auto-scaling-clusters.md).

mod config;
mod loop_;
mod observability;
mod policy;

pub use config::ScalerConfig;
pub use loop_::{ScalerState, TickMetrics, run_one_tick};
pub use observability::record_scale_decision;
pub use policy::{PolicyInputs, ScaleReason, ScalerDecision, decide};
