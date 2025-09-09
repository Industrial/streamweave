use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;
use std::ops::Range;
use std::time::Duration;

pub struct RandomNumberProducer {
  pub range: Range<i32>,
  pub count: Option<usize>,
  pub interval: Duration,
  pub config: ProducerConfig<i32>,
}

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
