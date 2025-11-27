use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;
use std::time::Duration;

/// A producer that emits items at regular intervals.
///
/// This producer emits `()` values at the specified interval, useful for
/// creating time-based triggers in pipelines.
pub struct IntervalProducer {
  /// The interval duration between emissions.
  pub interval: Duration,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<()>,
}

impl IntervalProducer {
  /// Creates a new `IntervalProducer` with the given interval.
  ///
  /// # Arguments
  ///
  /// * `interval` - The duration between emissions.
  pub fn new(interval: Duration) -> Self {
    Self {
      interval,
      config: ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this producer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<()>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this producer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}
