use crate::error::ErrorStrategy;
use crate::consumer::ConsumerConfig;
use tokio::sync::mpsc::Sender;

pub struct ChannelConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub channel: Option<Sender<T>>,
  pub config: ConsumerConfig<T>,
}

impl<T> ChannelConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new(sender: Sender<T>) -> Self {
    Self {
      channel: Some(sender),
      config: ConsumerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }
}
