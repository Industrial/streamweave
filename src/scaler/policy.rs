//! Policy engine: decides target shard count from metrics and config.
//!
//! The caller (scaler loop) is responsible for tracking stabilization (how long
//! scale-up/scale-down conditions have been satisfied) and cooldown (time since
//! last scale). This module is pure: given config, current state, and those
//! booleans, it returns the target shard count.

use crate::scaler::ScalerConfig;
use std::fmt;

/// Result of a scaling decision: target shard count and optional reason.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScalerDecision {
    /// Desired total shards (always within config min/max).
    pub target_shards: u32,
    /// Reason for the decision, if we are scaling (for observability).
    pub reason: Option<ScaleReason>,
}

/// Why we scaled (for logging/metrics).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScaleReason {
    /// Scaled up (e.g. throughput or backlog above threshold).
    ScaleUp,
    /// Scaled down (e.g. throughput and backlog below thresholds).
    ScaleDown,
}

impl fmt::Display for ScaleReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScaleReason::ScaleUp => write!(f, "scale_up"),
            ScaleReason::ScaleDown => write!(f, "scale_down"),
        }
    }
}

/// Inputs for the policy decision.
///
/// The scaler loop should populate this from metrics and its own stabilization state.
#[derive(Clone, Debug)]
pub struct PolicyInputs {
    /// Current number of shards.
    pub current_shards: u32,
    /// Aggregate items per second (e.g. rate of streamweave_items_in_total).
    pub items_per_sec: f64,
    /// Aggregate backlog (e.g. sum of streamweave_backlog_size), or 0 if not used.
    pub backlog: u64,
    /// True if scale-up condition has been satisfied for at least scale_up_stabilization_secs.
    pub scale_up_stabilized: bool,
    /// True if scale-down condition has been satisfied for at least scale_down_stabilization_secs.
    pub scale_down_stabilized: bool,
    /// Seconds since the last scale decision (for cooldown).
    pub seconds_since_last_scale: u64,
}

/// Decides target shard count from config and current inputs.
///
/// The caller sets `scale_up_stabilized` / `scale_down_stabilized` when the
/// corresponding condition (throughput or backlog vs thresholds) has held for
/// the config's stabilization window. Cooldown is enforced: no scale unless
/// `seconds_since_last_scale >= config.cooldown_secs`.
pub fn decide(config: &ScalerConfig, inputs: &PolicyInputs) -> ScalerDecision {
    let current = inputs.current_shards;
    if inputs.seconds_since_last_scale < config.cooldown_secs {
        return ScalerDecision {
            target_shards: current,
            reason: None,
        };
    }
    if inputs.scale_up_stabilized {
        let next = current.saturating_add(1);
        let target = config.clamp_shards(next);
        if target > current {
            return ScalerDecision {
                target_shards: target,
                reason: Some(ScaleReason::ScaleUp),
            };
        }
    }
    if inputs.scale_down_stabilized && current > config.min_shards {
        let next = current.saturating_sub(1);
        let target = config.clamp_shards(next);
        if target < current {
            return ScalerDecision {
                target_shards: target,
                reason: Some(ScaleReason::ScaleDown),
            };
        }
    }
    ScalerDecision {
        target_shards: current,
        reason: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{decide, PolicyInputs, ScaleReason, ScalerConfig};

    #[test]
    fn no_change_when_cooldown_active() {
        let config = ScalerConfig::default();
        let inputs = PolicyInputs {
            current_shards: 3,
            items_per_sec: 50_000.0,
            backlog: 100_000,
            scale_up_stabilized: true,
            scale_down_stabilized: false,
            seconds_since_last_scale: 30,
        };
        let d = decide(&config, &inputs);
        assert_eq!(d.target_shards, 3);
        assert!(d.reason.is_none());
    }

    #[test]
    fn scale_up_when_stabilized_and_above_threshold() {
        let config = ScalerConfig::default();
        let inputs = PolicyInputs {
            current_shards: 2,
            items_per_sec: 15_000.0,
            backlog: 0,
            scale_up_stabilized: true,
            scale_down_stabilized: false,
            seconds_since_last_scale: 120,
        };
        let d = decide(&config, &inputs);
        assert_eq!(d.target_shards, 3);
        assert_eq!(d.reason, Some(ScaleReason::ScaleUp));
    }

    #[test]
    fn scale_down_when_stabilized_and_below_threshold() {
        let config = ScalerConfig::default();
        let inputs = PolicyInputs {
            current_shards: 5,
            items_per_sec: 500.0,
            backlog: 1_000,
            scale_up_stabilized: false,
            scale_down_stabilized: true,
            seconds_since_last_scale: 400,
        };
        let d = decide(&config, &inputs);
        assert_eq!(d.target_shards, 4);
        assert_eq!(d.reason, Some(ScaleReason::ScaleDown));
    }

    #[test]
    fn respects_max_shards() {
        let mut config = ScalerConfig::new(1, 3);
        config.scale_up_threshold_items_per_sec = Some(1.0);
        let inputs = PolicyInputs {
            current_shards: 3,
            items_per_sec: 50_000.0,
            backlog: 0,
            scale_up_stabilized: true,
            scale_down_stabilized: false,
            seconds_since_last_scale: 120,
        };
        let d = decide(&config, &inputs);
        assert_eq!(d.target_shards, 3);
    }

    #[test]
    fn respects_min_shards() {
        let config = ScalerConfig::default();
        let inputs = PolicyInputs {
            current_shards: 1,
            items_per_sec: 0.0,
            backlog: 0,
            scale_up_stabilized: false,
            scale_down_stabilized: true,
            seconds_since_last_scale: 400,
        };
        let d = decide(&config, &inputs);
        assert_eq!(d.target_shards, 1);
    }
}
