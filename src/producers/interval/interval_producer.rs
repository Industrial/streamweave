use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;
use std::time::Duration;

pub struct IntervalProducer {
  pub interval: Duration,
  pub config: ProducerConfig<()>,
}

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
