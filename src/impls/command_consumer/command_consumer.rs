use crate::error::ErrorStrategy;
use crate::structs::command_consumer::CommandConsumer;
use crate::traits::consumer::ConsumerConfig;
use tokio::process::Command;

impl<T> CommandConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  pub fn new(command: String, args: Vec<String>) -> Self {
    let mut cmd = Command::new(command);
    cmd.args(args);
    Self {
      command: Some(cmd),
      config: ConsumerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }
}
