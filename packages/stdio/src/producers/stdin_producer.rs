use streamweave::ProducerConfig;
use streamweave_error::ErrorStrategy;

/// A producer that reads lines from standard input (stdin).
///
/// This producer reads lines from stdin and emits each line as a string item.
/// It's useful for building command-line tools that process input from stdin.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave_stdio::StdinProducer;
/// use streamweave::Pipeline;
///
/// let producer = StdinProducer::new();
/// let pipeline = Pipeline::new()
///     .producer(producer)
///     .transformer(/* ... */)
///     .consumer(/* ... */);
/// ```
pub struct StdinProducer {
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<String>,
}

impl StdinProducer {
  /// Creates a new `StdinProducer` with default configuration.
  pub fn new() -> Self {
    Self {
      config: ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this producer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
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

impl Default for StdinProducer {
  fn default() -> Self {
    Self::new()
  }
}
