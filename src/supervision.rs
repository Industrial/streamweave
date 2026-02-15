//! Actor-style supervision for node failure handling.
//!
//! Defines policies for what to do when a node panics or returns a fatal error:
//! restart the node, restart a group, stop, or escalate to the parent. See
//! [actor-supervision-trees.md](../docs/actor-supervision-trees.md).

use std::time::Duration;

/// Report of a node task failure sent to the supervisor channel.
///
/// The execution layer sends this when a node returns `Err` from `execute` or
/// when the node task panics (via `JoinError`).
#[derive(Clone, Debug)]
pub struct FailureReport {
    /// Node that failed.
    pub node_id: String,
    /// Error description (from `NodeExecutionError` or panic message).
    pub error: String,
}

/// Action to take when a supervised node (or subgraph) fails.
///
/// Used by the execution layer: on node panic or fatal `execute` error, the
/// supervisor applies the configured action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureAction {
    /// Restart this node (or subgraph) with fresh state. Optionally limited by `max_restarts`.
    Restart,
    /// Restart this node and its siblings (same parent).
    RestartGroup,
    /// Do not restart; stop this unit and notify parent.
    Stop,
    /// Notify parent supervisor; parent applies its own policy (e.g. graph stops).
    Escalate,
}

/// Policy for supervising a node or subgraph.
///
/// When a failure is detected (panic or `Err` from `execute`), the supervisor
/// applies `on_failure` subject to `max_restarts` and `restart_backoff`.
#[derive(Clone, Debug)]
pub struct SupervisionPolicy {
    /// Action to take on failure.
    pub on_failure: FailureAction,
    /// Maximum restarts before escalating or stopping. None = unbounded (use with care).
    pub max_restarts: Option<u32>,
    /// Delay between restart attempts (backoff).
    pub restart_backoff: Duration,
}

impl SupervisionPolicy {
    /// Creates a new policy with the given action and defaults.
    pub fn new(on_failure: FailureAction) -> Self {
        Self {
            on_failure,
            max_restarts: Some(3),
            restart_backoff: Duration::from_secs(1),
        }
    }

    /// Sets max restarts (None = unbounded).
    pub fn with_max_restarts(mut self, n: Option<u32>) -> Self {
        self.max_restarts = n;
        self
    }

    /// Sets restart backoff.
    pub fn with_restart_backoff(mut self, d: Duration) -> Self {
        self.restart_backoff = d;
        self
    }
}

impl Default for SupervisionPolicy {
    fn default() -> Self {
        Self {
            on_failure: FailureAction::Restart,
            max_restarts: Some(3),
            restart_backoff: Duration::from_secs(1),
        }
    }
}
