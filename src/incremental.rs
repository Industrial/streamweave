//! Types and helpers for incremental recomputation.
//!
//! Supports progress tracking and dependency-based time-range recomputation.
//! See [incremental-recomputation.md](../docs/incremental-recomputation.md).
//!
//! ## Time-scoped execution API
//!
//! The intended API for running only a subset of nodes for a time range:
//!
//! - [`plan_recompute`]: Given a [`RecomputeRequest`] and sink frontiers, returns
//!   which nodes need recomputation.
//! - `Graph::execute_for_time_range(nodes, time_range)`: Runs only the given nodes
//!   for the specified time range. Requires execution-model support (subgraph
//!   extraction, time-filtered inputs). See task streamweave-ulx.4.
//!
//! For now, use [`plan_recompute`] with [`Graph::nodes_downstream_transitive`]
//! and [`execute_recompute`](crate::graph::Graph::execute_recompute) which runs
//! the full graph as a fallback when subgraph execution is not yet implemented.

use crate::time::LogicalTime;
use std::collections::{HashMap, HashSet};

/// A closed time range [from, to] for recomputation scope.
///
/// Used when requesting "recompute output for logical times in this range."
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
    /// If set, only nodes downstream of this node (inclusive) are considered.
    pub source_node: Option<String>,
}

impl RecomputeRequest {
    /// Creates a new recompute request.
    pub fn new(time_range: TimeRange, source_node: Option<String>) -> Self {
        Self {
            time_range,
            source_node,
        }
    }
}

/// Plan describing which nodes need recomputation for a [`RecomputeRequest`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecomputePlan {
    /// Node IDs that need to run for the requested time range.
    /// Topological order (sources first) is recommended for execution.
    pub nodes: Vec<String>,
}

/// Computes which nodes need recomputation for the given request.
///
/// Uses `downstream_fn` (e.g. `Graph::nodes_downstream_transitive`) to determine
/// candidates. When `sink_frontiers` and `sink_keys` are provided, sinks that have
/// already completed output for the entire time range may be excluded (optimization).
///
/// # Arguments
///
/// * `request` - The recompute request (time range and optional source node)
/// * `downstream_fn` - Returns transitive downstream nodes for a given node
/// * `all_nodes` - All node names (used when `source_node` is None)
/// * `sink_frontiers` - Optional map from (node, port) to completed logical time
/// * `sink_keys` - Optional set of (node, port) that are sink ports (exposed outputs)
///
/// # Returns
///
/// The set of nodes to recompute, including the source when specified.
#[allow(clippy::type_complexity)]
pub fn plan_recompute<F>(
    request: &RecomputeRequest,
    downstream_fn: F,
    all_nodes: &[String],
    sink_frontiers: Option<&HashMap<(String, String), LogicalTime>>,
    sink_keys: Option<&HashSet<(String, String)>>,
) -> RecomputePlan
where
    F: Fn(&str) -> Vec<String>,
{
    let mut candidates: Vec<String> = if let Some(ref src) = request.source_node {
        let mut v = vec![src.clone()];
        v.extend(downstream_fn(src));
        v.sort();
        v.dedup();
        v
    } else {
        let mut v = all_nodes.to_vec();
        v.sort();
        v
    };

    if let (Some(frontiers), Some(keys)) = (sink_frontiers, sink_keys) {
        candidates.retain(|n| {
            let node_sink_keys: Vec<_> = keys.iter().filter(|(node, _)| node == n).collect();
            if node_sink_keys.is_empty() {
                return true;
            }
            let all_complete = node_sink_keys.iter().all(|(node, port)| {
                frontiers
                    .get(&((*node).clone(), (*port).clone()))
                    .map_or(false, |&t| t >= request.time_range.to)
            });
            !all_complete
        });
    }
    candidates.sort();
    RecomputePlan { nodes: candidates }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn time_range_contains() {
        let r = TimeRange::new(LogicalTime::new(10), LogicalTime::new(20));
        assert!(r.contains(LogicalTime::new(10)));
        assert!(r.contains(LogicalTime::new(15)));
        assert!(r.contains(LogicalTime::new(20)));
        assert!(!r.contains(LogicalTime::new(9)));
        assert!(!r.contains(LogicalTime::new(21)));
    }

    #[test]
    fn plan_recompute_all_nodes_when_no_source() {
        let req = RecomputeRequest::new(
            TimeRange::new(LogicalTime::new(1), LogicalTime::new(10)),
            None,
        );
        let all = vec!["a".into(), "b".into(), "c".into()];
        let plan = plan_recompute(&req, |_| vec![], &all, None, None);
        assert_eq!(plan.nodes, vec!["a", "b", "c"]);
    }

    #[test]
    fn plan_recompute_source_and_downstream() {
        let req = RecomputeRequest::new(
            TimeRange::new(LogicalTime::new(1), LogicalTime::new(10)),
            Some("a".into()),
        );
        let all = vec!["a".into(), "b".into(), "c".into()];
        let downstream = |n: &str| match n {
            "a" => vec!["b".into(), "c".into()],
            "b" => vec!["c".into()],
            _ => vec![],
        };
        let plan = plan_recompute(&req, downstream, &all, None, None);
        assert_eq!(plan.nodes, vec!["a", "b", "c"]);
    }

    #[test]
    fn plan_recompute_excludes_sink_when_frontier_complete() {
        let req = RecomputeRequest::new(
            TimeRange::new(LogicalTime::new(1), LogicalTime::new(10)),
            Some("a".into()),
        );
        let all = vec!["a".into(), "b".into()];
        let downstream = |n: &str| if n == "a" { vec!["b".into()] } else { vec![] };
        let mut frontiers = HashMap::new();
        frontiers.insert(
            ("b".to_string(), "out".to_string()),
            LogicalTime::new(15),
        );
        let mut keys = HashSet::new();
        keys.insert(("b".to_string(), "out".to_string()));
        let plan = plan_recompute(&req, downstream, &all, Some(&frontiers), Some(&keys));
        assert_eq!(plan.nodes, vec!["a"]);
    }
}
