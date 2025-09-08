use crate::error::ErrorStrategy;
use crate::structs::transformers::interleave::InterleaveTransformer;
use crate::traits::transformer::TransformerConfig;
use futures::Stream;
use std::pin::Pin;

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
