use crate::error::ErrorStrategy;
use crate::structs::transformers::flat_map::FlatMapTransformer;
use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;

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
