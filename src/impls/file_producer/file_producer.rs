use crate::error::ErrorStrategy;
use crate::structs::file_producer::FileProducer;

impl FileProducer {
  pub fn new(path: String) -> Self {
    Self {
      path,
      config: crate::traits::producer::ProducerConfig::default(),
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
