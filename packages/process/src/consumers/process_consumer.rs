use streamweave::ConsumerConfig;
use streamweave_error::ErrorStrategy;

/// A consumer that sends items to an external process's stdin.
pub struct ProcessConsumer {
  /// The command to execute
  pub command: String,
  /// Command arguments
  pub args: Vec<String>,
  /// Configuration for the consumer
  pub config: ConsumerConfig<String>,
}

impl ProcessConsumer {
  /// Creates a new `ProcessConsumer` with the given command.
  ///
  /// # Arguments
  ///
  /// * `command` - The command to execute
  pub fn new(command: String) -> Self {
    Self {
      command,
      args: Vec::new(),
      config: ConsumerConfig::default(),
    }
  }

  /// Adds an argument to the command.
  pub fn arg(mut self, arg: String) -> Self {
    self.args.push(arg);
    self
  }

  /// Sets the error handling strategy.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this consumer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }
}
