use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use std::marker::PhantomData;

pub struct ChunkTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub size: usize,
  pub config: TransformerConfig<T>,
  pub _phantom: PhantomData<T>,
}

impl<T> ChunkTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new(size: usize) -> Self {
    Self {
      size,
      config: TransformerConfig::default(),
      _phantom: PhantomData,
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
