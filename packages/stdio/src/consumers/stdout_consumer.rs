use streamweave::ConsumerConfig;
use streamweave_error::ErrorStrategy;

/// A consumer that writes items to standard output (stdout).
///
/// This consumer writes each item from the stream to stdout, one per line.
/// It's useful for building command-line tools that output to stdout.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave_stdio::StdoutConsumer;
/// use streamweave::Pipeline;
///
/// let consumer = StdoutConsumer::new();
/// let pipeline = Pipeline::new()
///     .producer(/* ... */)
///     .transformer(/* ... */)
///     .consumer(consumer);
/// ```
pub struct StdoutConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<T>,
}

impl<T> StdoutConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  /// Creates a new `StdoutConsumer` with default configuration.
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

impl<T> Default for StdoutConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}
