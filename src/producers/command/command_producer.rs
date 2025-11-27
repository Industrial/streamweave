use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;

/// A producer that executes shell commands and produces their output.
///
/// This producer runs a command with the specified arguments and emits
/// each line of output as a separate item.
pub struct CommandProducer {
  /// The command to execute.
  pub command: String,
  /// The arguments to pass to the command.
  pub args: Vec<String>,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<String>,
}

impl CommandProducer {
  /// Creates a new `CommandProducer` with the given command and arguments.
  ///
  /// # Arguments
  ///
  /// * `command` - The command to execute.
  /// * `args` - The arguments to pass to the command.
  pub fn new(command: impl Into<String>, args: Vec<impl Into<String>>) -> Self {
    Self {
      command: command.into(),
      args: args.into_iter().map(|arg| arg.into()).collect(),
      config: crate::producer::ProducerConfig::default(),
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
