use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;
use std::ops::Range;
use std::time::Duration;

/// A producer that generates random numbers within a specified range.
///
/// This producer emits random integers within the given range at regular intervals.
pub struct RandomNumberProducer {
  /// The range of random numbers to generate.
  pub range: Range<i32>,
  /// Optional limit on the number of random numbers to generate.
  pub count: Option<usize>,
  /// The interval between random number emissions.
  pub interval: Duration,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<i32>,
}

impl RandomNumberProducer {
  /// Creates a new `RandomNumberProducer` with the given range.
  ///
  /// # Arguments
  ///
  /// * `range` - The range of random numbers to generate.
  pub fn new(range: Range<i32>) -> Self {
    Self {
      range,
      count: None,
      interval: Duration::from_millis(100),
      config: ProducerConfig::default(),
    }
  }

  /// Creates a new `RandomNumberProducer` with the given range and count limit.
  ///
  /// # Arguments
  ///
  /// * `range` - The range of random numbers to generate.
  /// * `count` - The maximum number of random numbers to generate.
  pub fn with_count(range: Range<i32>, count: usize) -> Self {
    Self {
      range,
      count: Some(count),
      interval: Duration::from_millis(100),
      config: ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this producer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<i32>) -> Self {
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
