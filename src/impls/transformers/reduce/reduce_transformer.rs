use crate::error::ErrorStrategy;
use crate::structs::transformers::reduce::ReduceTransformer;
use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;

impl<T, Acc, F> ReduceTransformer<T, Acc, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Acc: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(Acc, T) -> Acc + Send + Clone + 'static,
{
  pub fn new(initial: Acc, reducer: F) -> Self {
    Self {
      accumulator: initial,
      reducer,
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
