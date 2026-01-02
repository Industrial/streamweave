//! Control flow tests
//!
//! This module provides comprehensive tests for control flow constructs including
//! routers (If, ErrorBranch, Match), transformers (ForEach, Delay, Timeout, While,
//! Join, Aggregate, GroupBy), and aggregators (Sum, Count, Min, Max).
//!
//! Uses both async tokio tests for integration testing and proptest for property-based testing.

mod common;

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
