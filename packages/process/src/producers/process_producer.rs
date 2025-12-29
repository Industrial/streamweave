use streamweave::ProducerConfig;
use streamweave_error::ErrorStrategy;

/// A producer that reads output from an external process.
///
/// Spawns a process and emits its stdout line by line.
pub struct ProcessProducer {
  /// The command to execute
  pub command: String,
  /// Command arguments
  pub args: Vec<String>,
  /// Configuration for the producer
  pub config: ProducerConfig<String>,
}

impl ProcessProducer {
  /// Creates a new `ProcessProducer` with the given command.
  ///
  /// # Arguments
  ///
  /// * `command` - The command to execute
  pub fn new(command: String) -> Self {
    Self {
      command,
      args: Vec::new(),
      config: ProducerConfig::default(),
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

  /// Sets the name for this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}
