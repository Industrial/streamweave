use streamweave::ConsumerConfig;
use streamweave_error::ErrorStrategy;

/// A consumer that writes items to standard error (stderr).
///
/// This consumer writes each item from the stream to stderr, one per line.
/// It's useful for building command-line tools that output errors or diagnostics to stderr.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave_stdio::StderrConsumer;
/// use streamweave::Pipeline;
///
/// let consumer = StderrConsumer::new();
/// let pipeline = Pipeline::new()
///     .producer(/* ... */)
///     .transformer(/* ... */)
///     .consumer(consumer);
/// ```
pub struct StderrConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<T>,
}

impl<T> StderrConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  /// Creates a new `StderrConsumer` with default configuration.
  pub fn new() -> Self {
    Self {
      config: ConsumerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this consumer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this consumer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this consumer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }
}

impl<T> Default for StderrConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}
