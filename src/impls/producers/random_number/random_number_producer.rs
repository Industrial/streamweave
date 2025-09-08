use crate::error::ErrorStrategy;
use crate::structs::producers::random_number::RandomNumberProducer;
use crate::traits::producer::ProducerConfig;
use std::ops::Range;
use std::time::Duration;

impl RandomNumberProducer {
  pub fn new(range: Range<i32>) -> Self {
    Self {
      range,
      count: None,
      interval: Duration::from_millis(100),
      config: ProducerConfig::default(),
    }
  }

  pub fn with_count(range: Range<i32>, count: usize) -> Self {
    Self {
      range,
      count: Some(count),
      interval: Duration::from_millis(100),
      config: ProducerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<i32>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}
