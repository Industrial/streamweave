//! Observability for the built-in scaler: log and record scale decisions.

use crate::scaler::ScaleReason;
use metrics::counter;
use tracing::info;

/// Records a scale decision for logs and metrics.
///
/// Call this when you apply a scale decision (e.g. after `run_one_tick` returns
/// a decision with `reason.is_some()` and you call `scale_to(decision.target_shards)`).
///
/// - **Log:** `info!` with reason, from_shards, target_shards.
/// - **Metric:** `streamweave_scaler_scale_total` counter with label `reason` (scale_up | scale_down).
/// - **Metric:** `streamweave_scaler_target_shards` gauge set to target (for dashboards).
pub fn record_scale_decision(reason: ScaleReason, from_shards: u32, target_shards: u32) {
  info!(
      %reason,
      from_shards,
      target_shards,
      "scaler applied scale decision"
  );
  counter!(
      "streamweave_scaler_scale_total",
      "reason" => reason.to_string()
  )
  .increment(1);
  metrics::gauge!("streamweave_scaler_target_shards").set(target_shards as f64);
}
