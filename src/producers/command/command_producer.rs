use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;

pub struct CommandProducer {
  pub command: String,
  pub args: Vec<String>,
  pub config: ProducerConfig<String>,
}

impl CommandProducer {
  pub fn new(command: impl Into<String>, args: Vec<impl Into<String>>) -> Self {
    Self {
      command: command.into(),
      args: args.into_iter().map(|arg| arg.into()).collect(),
      config: crate::producer::ProducerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}
