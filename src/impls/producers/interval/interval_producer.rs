use crate::error::ErrorStrategy;
use crate::structs::producers::interval::IntervalProducer;
use crate::traits::producer::ProducerConfig;
use std::time::Duration;

impl IntervalProducer {
  pub fn new(interval: Duration) -> Self {
    Self {
      interval,
      config: ProducerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<()>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}
