use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use std::marker::PhantomData;

pub struct FlatMapTransformer<F, I, O>
where
  F: Fn(I) -> Vec<O> + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub f: F,
  pub config: TransformerConfig<I>,
  pub _phantom_i: PhantomData<I>,
  pub _phantom_o: PhantomData<O>,
}

impl<F, I, O> FlatMapTransformer<F, I, O>
where
  F: Fn(I) -> Vec<O> + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new(f: F) -> Self {
    Self {
      f,
      config: TransformerConfig::default(),
      _phantom_i: PhantomData,
      _phantom_o: PhantomData,
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<I>) -> Self {
    self.config = self.config.with_error_strategy(strategy);
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config = self.config.with_name(name);
    self
  }
}
