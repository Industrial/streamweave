use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;

/// A producer that yields environment variables as key-value pairs.
///
/// This producer reads environment variables and emits them as `(String, String)`
/// tuples. Optionally, it can filter to only specific variable names.
#[derive(Debug, Clone)]
pub struct EnvVarProducer {
  /// Optional filter to only include specific environment variable names.
  pub filter: Option<Vec<String>>,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<(String, String)>,
}

impl EnvVarProducer {
  /// Creates a new `EnvVarProducer` that produces all environment variables.
  pub fn new() -> Self {
    Self {
      filter: None,
      config: ProducerConfig::default(),
    }
  }

  /// Creates a new `EnvVarProducer` that only produces the specified environment variables.
  ///
  /// # Arguments
  ///
  /// * `vars` - The list of environment variable names to include.
  pub fn with_vars(vars: Vec<String>) -> Self {
    Self {
      filter: Some(vars),
      config: ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this producer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<(String, String)>) -> Self {
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

impl Default for EnvVarProducer {
  fn default() -> Self {
    Self::new()
  }
}
