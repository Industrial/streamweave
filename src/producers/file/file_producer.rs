use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;

pub struct FileProducer {
  pub path: String,
  pub config: ProducerConfig<String>,
}

impl FileProducer {
  pub fn new(path: String) -> Self {
    Self {
      path,
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
