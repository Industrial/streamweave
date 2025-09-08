use crate::error::ErrorStrategy;
use crate::structs::transformers::map::MapTransformer;
use crate::traits::transformer::TransformerConfig;

impl<F, I, O> MapTransformer<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new(f: F) -> Self {
    Self {
      f,
      config: TransformerConfig::default(),
      _phantom_i: std::marker::PhantomData,
      _phantom_o: std::marker::PhantomData,
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<I>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<F, I, O> Clone for MapTransformer<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      f: self.f.clone(),
      config: self.config.clone(),
      _phantom_i: std::marker::PhantomData,
      _phantom_o: std::marker::PhantomData,
    }
  }
}
