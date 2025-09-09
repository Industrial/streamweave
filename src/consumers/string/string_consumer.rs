use crate::consumer::ConsumerConfig;
use crate::error::ErrorStrategy;

pub struct StringConsumer {
  pub buffer: String,
  pub config: ConsumerConfig<String>,
}

impl Default for StringConsumer {
  fn default() -> Self {
    Self::new()
  }
}

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
