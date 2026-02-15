//! Configuration for the built-in scaler.
//!
//! Defines min/max shards, scale-up/scale-down thresholds (throughput and/or
//! backlog), stabilization windows, and cooldown to avoid flapping.

/// Configuration for the built-in scaling policy.
///
/// Aligns with common autoscaler expectations (e.g. Kubernetes HPA):
/// bounded scale range, separate scale-up vs scale-down thresholds and
/// stabilization windows, and a cooldown between scale decisions.
#[derive(Clone, Debug)]
pub struct ScalerConfig {
    /// Minimum number of shards (never scale below this).
    pub min_shards: u32,
    /// Maximum number of shards (never scale above this).
    pub max_shards: u32,
    /// Scale up when items/sec (aggregate) exceeds this for the scale-up stabilization window.
    pub scale_up_threshold_items_per_sec: Option<f64>,
    /// Scale down when items/sec is below this for the scale-down stabilization window.
    pub scale_down_threshold_items_per_sec: Option<f64>,
    /// Scale up when backlog (e.g. sum of streamweave_backlog_size) exceeds this.
    pub scale_up_threshold_backlog: Option<u64>,
    /// Scale down when backlog is below this for the scale-down stabilization window.
    pub scale_down_threshold_backlog: Option<u64>,
    /// Only scale up if the scale-up condition held for at least this many seconds.
    pub scale_up_stabilization_secs: u64,
    /// Only scale down if the scale-down condition held for at least this many seconds.
    pub scale_down_stabilization_secs: u64,
    /// Minimum seconds between any scale decision (up or down).
    pub cooldown_secs: u64,
}

impl Default for ScalerConfig {
    fn default() -> Self {
        Self {
            min_shards: 1,
            max_shards: 10,
            scale_up_threshold_items_per_sec: Some(10_000.0),
            scale_down_threshold_items_per_sec: Some(1_000.0),
            scale_up_threshold_backlog: Some(50_000),
            scale_down_threshold_backlog: Some(5_000),
            scale_up_stabilization_secs: 30,
            scale_down_stabilization_secs: 300,
            cooldown_secs: 60,
        }
    }
}

impl ScalerConfig {
    /// Creates a new config with the given shard bounds.
    pub fn new(min_shards: u32, max_shards: u32) -> Self {
        Self {
            min_shards,
            max_shards,
            ..Default::default()
        }
    }

    /// Validates the config: min_shards <= max_shards, stabilization and cooldown non-zero when used.
    pub fn validate(&self) -> Result<(), String> {
        if self.min_shards > self.max_shards {
            return Err(format!(
                "min_shards ({}) must be <= max_shards ({})",
                self.min_shards, self.max_shards
            ));
        }
        if self.scale_up_threshold_items_per_sec.is_some()
            || self.scale_up_threshold_backlog.is_some()
        {
            if self.scale_up_stabilization_secs == 0 {
                return Err("scale_up_stabilization_secs must be > 0 when scale-up thresholds are set".to_string());
            }
        }
        if self.scale_down_threshold_items_per_sec.is_some()
            || self.scale_down_threshold_backlog.is_some()
        {
            if self.scale_down_stabilization_secs == 0 {
                return Err(
                    "scale_down_stabilization_secs must be > 0 when scale-down thresholds are set"
                        .to_string(),
                );
            }
        }
        if self.cooldown_secs == 0 {
            return Err("cooldown_secs must be > 0".to_string());
        }
        Ok(())
    }

    /// Clamps the given shard count to [min_shards, max_shards].
    pub fn clamp_shards(&self, n: u32) -> u32 {
        n.clamp(self.min_shards, self.max_shards)
    }
}

#[cfg(test)]
mod tests {
    use super::ScalerConfig;

    #[test]
    fn default_validates() {
        let c = ScalerConfig::default();
        assert!(c.validate().is_ok());
        assert_eq!(c.clamp_shards(0), 1);
        assert_eq!(c.clamp_shards(100), 10);
    }

    #[test]
    fn validate_min_max() {
        let mut c = ScalerConfig::default();
        c.min_shards = 5;
        c.max_shards = 3;
        assert!(c.validate().is_err());
    }

    #[test]
    fn validate_cooldown() {
        let mut c = ScalerConfig::default();
        c.cooldown_secs = 0;
        assert!(c.validate().is_err());
    }
}
