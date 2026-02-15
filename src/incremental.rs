//! Types and helpers for incremental recomputation.
//!
//! Supports progress tracking and dependency-based time-range recomputation.
//! See [incremental-recomputation.md](../docs/incremental-recomputation.md).

use crate::time::LogicalTime;

/// A closed time range [from, to] for recomputation scope.
///
/// Used when requesting "recompute output for logical times in this range."
/// Full execution-model support for time-scoped recomputation is planned.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TimeRange {
    /// Start of the range (inclusive).
    pub from: LogicalTime,
    /// End of the range (inclusive).
    pub to: LogicalTime,
}

impl TimeRange {
    /// Creates a time range.
    pub fn new(from: LogicalTime, to: LogicalTime) -> Self {
        Self { from, to }
    }

    /// Returns true if the range contains the given time.
    pub fn contains(&self, t: LogicalTime) -> bool {
        t >= self.from && t <= self.to
    }
}

/// Request to recompute output for a time range.
///
/// When the execution model supports time-scoped execution, this describes
/// "recompute only the minimal set of nodes for times in `time_range`."
/// Optionally restrict to nodes downstream of `source_node`.
#[derive(Clone, Debug)]
pub struct RecomputeRequest {
    /// The time range to recompute.
    pub time_range: TimeRange,
    /// If set, only nodes downstream of this node are considered.
    pub source_node: Option<String>,
}
