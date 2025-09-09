use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;

pub struct VecProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub data: Vec<T>,
  pub config: ProducerConfig<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> VecProducer<T> {
  pub fn new(data: Vec<T>) -> Self {
    Self {
      data,
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
