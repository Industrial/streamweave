use crate::consumer::ConsumerConfig;
use crate::error::ErrorStrategy;
use tokio::fs::File;

pub struct FileConsumer {
  pub file: Option<File>,
  pub path: String,
  pub config: ConsumerConfig<String>,
}

impl FileConsumer {
  pub fn new(path: String) -> Self {
    Self {
      file: None,
      path,
      config: ConsumerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }
}
