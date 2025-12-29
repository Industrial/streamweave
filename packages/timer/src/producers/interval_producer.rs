use std::time::Duration;
use streamweave::ProducerConfig;
use streamweave_error::ErrorStrategy;

/// A producer that emits events at regular intervals.
///
/// This producer generates timestamp events at the specified interval duration.
pub struct IntervalProducer {
  /// The interval duration between events
  pub interval: Duration,
  /// Configuration for the producer
  pub config: ProducerConfig<std::time::SystemTime>,
}

impl IntervalProducer {
  /// Creates a new `IntervalProducer` with the given interval.
  ///
  /// # Arguments
  ///
  /// * `interval` - The duration between emitted events
  pub fn new(interval: Duration) -> Self {
    Self {
      interval,
      config: ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<std::time::SystemTime>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}
