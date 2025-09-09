use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;
use tokio::sync::mpsc;

pub struct ChannelProducer<T: std::fmt::Debug + Clone + Send + Sync> {
  pub rx: mpsc::Receiver<T>,
  pub config: ProducerConfig<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync> ChannelProducer<T> {
  pub fn new(rx: mpsc::Receiver<T>) -> Self {
    Self {
      rx,
      config: crate::producer::ProducerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}
