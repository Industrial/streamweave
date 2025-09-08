use crate::error::ErrorStrategy;
use crate::structs::producers::string::StringProducer;

impl StringProducer {
  pub fn new(data: String, chunk_size: usize) -> Self {
    Self {
      data,
      chunk_size,
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
