//! Stateful scaler tick: maintains stabilization state and runs the policy.
//!
//! The caller runs in a loop: each interval, obtain `current_shards` (e.g. from
//! `RebalanceCoordinator::current_assignment().await.total_shards`), obtain
//! metrics (items/sec, backlog), then call [`run_one_tick`]. If the decision has
//! a reason, call `scale_to(decision.target_shards)` on your coordinator
//! (e.g. `InMemoryCoordinator::scale_to`). "Wait for rebalance" is left to the
//! coordinator/workers; the cooldown in config avoids scaling again too soon.

use super::config::ScalerConfig;
use super::policy::{decide, PolicyInputs, ScalerDecision};
use std::time::Instant;

/// State carried between ticks for stabilization and cooldown.
#[derive(Clone, Debug, Default)]
pub struct ScalerState {
    /// Last time we applied a scale decision.
    pub last_scale_at: Option<Instant>,
    /// When the scale-up condition became continuously true (None if currently false).
    pub scale_up_satisfied_since: Option<Instant>,
    /// When the scale-down condition became continuously true (None if currently false).
    pub scale_down_satisfied_since: Option<Instant>,
}

/// Metrics for one tick (caller supplies from Prometheus or in-memory counters).
#[derive(Clone, Copy, Debug)]
pub struct TickMetrics {
    /// Aggregate items per second.
    pub items_per_sec: f64,
    /// Aggregate backlog (e.g. sum of streamweave_backlog_size), or 0.
    pub backlog: u64,
}

/// Runs one scaler tick: updates stabilization state, runs the policy, returns decision and new state.
///
/// The caller should:
/// 1. Get `current_shards` from `RebalanceCoordinator::current_assignment().await.total_shards`.
/// 2. Get `metrics` (e.g. from Prometheus rate of streamweave_items_in_total, sum of streamweave_backlog_size).
/// 3. Call `run_one_tick(config, state, current_shards, metrics, now)`.
/// 4. If `decision.reason.is_some()`, call `InMemoryCoordinator::scale_to(decision.target_shards)` (or your coordinator’s equivalent).
/// 5. Use the returned `new_state` for the next tick.
pub fn run_one_tick(
    config: &ScalerConfig,
    state: &ScalerState,
    current_shards: u32,
    metrics: TickMetrics,
    now: Instant,
) -> (ScalerDecision, ScalerState) {
    let scale_up_condition = (config.scale_up_threshold_items_per_sec.map_or(false, |t| {
        metrics.items_per_sec >= t
    })) || (config.scale_up_threshold_backlog.map_or(false, |t| metrics.backlog >= t));

    let scale_down_condition =
        (config.scale_down_threshold_items_per_sec.map_or(true, |t| {
            metrics.items_per_sec <= t
        })) && (config.scale_down_threshold_backlog.map_or(true, |t| metrics.backlog <= t));

    let scale_up_satisfied_since = if scale_up_condition {
        Some(state.scale_up_satisfied_since.unwrap_or(now))
    } else {
        None
    };

    let scale_down_satisfied_since = if scale_down_condition {
        Some(state.scale_down_satisfied_since.unwrap_or(now))
    } else {
        None
    };

    let scale_up_stabilized = scale_up_satisfied_since.map_or(false, |t| {
        now.duration_since(t).as_secs() >= config.scale_up_stabilization_secs
    });
    let scale_down_stabilized = scale_down_satisfied_since.map_or(false, |t| {
        now.duration_since(t).as_secs() >= config.scale_down_stabilization_secs
    });

    let seconds_since_last_scale = state
        .last_scale_at
        .map(|t| now.duration_since(t).as_secs())
        .unwrap_or(u64::MAX);

    let inputs = PolicyInputs {
        current_shards,
        items_per_sec: metrics.items_per_sec,
        backlog: metrics.backlog,
        scale_up_stabilized,
        scale_down_stabilized,
        seconds_since_last_scale,
    };

    let decision = decide(config, &inputs);

    let last_scale_at = if decision.reason.is_some() {
        Some(now)
    } else {
        state.last_scale_at
    };

    let new_state = ScalerState {
        last_scale_at,
        scale_up_satisfied_since,
        scale_down_satisfied_since,
    };

    (decision, new_state)
}

#[cfg(test)]
mod tests {
    use super::{run_one_tick, ScalerState, TickMetrics};
    use crate::rebalance::{InMemoryCoordinator, RebalanceCoordinator};
    use crate::scaler::{record_scale_decision, ScaleReason, ScalerConfig};
    use std::time::Instant;

    #[test]
    fn first_tick_scale_up_after_stabilization() {
        let config = ScalerConfig::default();
        let state = ScalerState::default();
        let now = Instant::now();
        let metrics = TickMetrics {
            items_per_sec: 20_000.0,
            backlog: 0,
        };
        // First tick: scale_up_satisfied_since = now, but stabilization not met yet.
        let (d, s1) = run_one_tick(&config, &state, 2, metrics, now);
        assert_eq!(d.target_shards, 2);
        assert!(d.reason.is_none());
        assert!(s1.scale_up_satisfied_since.is_some());

        // Simulate 60s later: stabilization (30s) and cooldown (60s) both met.
        let now2 = now + std::time::Duration::from_secs(60);
        let (d2, s2) = run_one_tick(&config, &s1, 2, metrics, now2);
        assert_eq!(d2.target_shards, 3);
        assert!(d2.reason.is_some());
        assert!(s2.last_scale_at.is_some());
    }

    #[tokio::test]
    async fn integration_tick_then_scale_to_updates_coordinator() {
        let coordinator = InMemoryCoordinator::new(0, 2);
        let config = ScalerConfig::default();
        let mut state = ScalerState::default();
        let now = Instant::now();
        let metrics = TickMetrics {
            items_per_sec: 25_000.0,
            backlog: 60_000,
        };
        // First tick: no scale (stabilization not met).
        let assignment = coordinator.current_assignment().await;
        let (decision, new_state) = run_one_tick(&config, &state, assignment.total_shards, metrics, now);
        state = new_state;
        assert_eq!(decision.target_shards, 2);
        assert!(decision.reason.is_none());

        // Advance time 90s and tick again: should scale up.
        let now2 = now + std::time::Duration::from_secs(90);
        let assignment2 = coordinator.current_assignment().await;
        let (decision2, new_state2) = run_one_tick(&config, &state, assignment2.total_shards, metrics, now2);
        state = new_state2;
        assert_eq!(decision2.target_shards, 3);
        assert_eq!(decision2.reason, Some(ScaleReason::ScaleUp));
        coordinator.scale_to(decision2.target_shards);
        if let Some(reason) = decision2.reason {
            record_scale_decision(reason, assignment2.total_shards, decision2.target_shards);
        }

        let assignment3 = coordinator.current_assignment().await;
        assert_eq!(assignment3.total_shards, 3);
    }
}
