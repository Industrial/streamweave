use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;
use num_traits::Num;

pub struct RangeProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Num + Copy + PartialOrd + 'static,
{
  pub start: T,
  pub end: T,
  pub step: T,
  pub config: ProducerConfig<T>,
}

impl<T> RangeProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Num + Copy + PartialOrd + 'static,
{
  pub fn new(start: T, end: T, step: T) -> Self {
    Self {
      start,
      end,
      step,
      config: ProducerConfig::default(),
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
