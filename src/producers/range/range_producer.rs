use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;
use num_traits::Num;

/// A producer that generates items in a numeric range.
///
/// This producer generates a sequence of values starting from `start`,
/// incrementing by `step`, until reaching `end`.
pub struct RangeProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Num + Copy + PartialOrd + 'static,
{
  /// The starting value of the range.
  pub start: T,
  /// The ending value of the range (exclusive).
  pub end: T,
  /// The step size for incrementing values.
  pub step: T,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<T>,
}

impl<T> RangeProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Num + Copy + PartialOrd + 'static,
{
  /// Creates a new `RangeProducer` with the given start, end, and step values.
  ///
  /// # Arguments
  ///
  /// * `start` - The starting value of the range.
  /// * `end` - The ending value of the range (exclusive).
  /// * `step` - The step size for incrementing values.
  pub fn new(start: T, end: T, step: T) -> Self {
    Self {
      start,
      end,
      step,
      config: ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this producer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
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
