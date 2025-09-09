use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use futures::Stream;
use std::marker::PhantomData;
use std::pin::Pin;

pub struct InterleaveTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub other: Pin<Box<dyn Stream<Item = T> + Send>>,
  pub config: TransformerConfig<T>,
  pub _phantom: PhantomData<T>,
}

impl<T> InterleaveTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new(other: Pin<Box<dyn Stream<Item = T> + Send>>) -> Self {
    Self {
      other,
      config: TransformerConfig::default(),
      _phantom: std::marker::PhantomData,
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
