use crate::error::ErrorStrategy;
use crate::structs::consumers::string::StringConsumer;
use crate::traits::consumer::ConsumerConfig;

impl StringConsumer {
  pub fn new() -> Self {
    Self {
      buffer: String::new(),
      config: ConsumerConfig::default(),
    }
  }

  pub fn with_capacity(capacity: usize) -> Self {
    Self {
      buffer: String::with_capacity(capacity),
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

  pub fn into_string(self) -> String {
    self.buffer
  }
}
