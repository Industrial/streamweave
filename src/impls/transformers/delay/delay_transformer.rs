use crate::error::ErrorStrategy;
use crate::structs::transformers::delay::DelayTransformer;
use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;
use std::time::Duration;

impl<T> DelayTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new(duration: Duration) -> Self {
    Self {
      duration,
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config = self.config.with_error_strategy(strategy);
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config = self.config.with_name(name);
    self
  }
}
