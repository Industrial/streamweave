use streamweave::ConsumerConfig;
use streamweave_error::ErrorStrategy;

/// A consumer that sets environment variables from key-value pairs.
///
/// This consumer takes `(String, String)` tuples from the stream and sets them
/// as environment variables. It's useful for configuring the runtime environment
/// based on stream data.
#[derive(Debug, Clone)]
pub struct EnvVarConsumer {
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<(String, String)>,
}

impl EnvVarConsumer {
  /// Creates a new `EnvVarConsumer`.
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
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<(String, String)>) -> Self {
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

impl Default for EnvVarConsumer {
  fn default() -> Self {
    Self::new()
  }
}
