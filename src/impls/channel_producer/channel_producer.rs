use crate::error::ErrorStrategy;
use crate::structs::channel_producer::ChannelProducer;
use tokio::sync::mpsc;

impl<T: std::fmt::Debug + Clone + Send + Sync> ChannelProducer<T> {
  pub fn new(rx: mpsc::Receiver<T>) -> Self {
    Self {
      rx,
      config: crate::traits::producer::ProducerConfig::default(),
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
