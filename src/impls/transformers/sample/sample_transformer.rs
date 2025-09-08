use crate::error::ErrorStrategy;
use crate::structs::transformers::sample::SampleTransformer;
use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;

impl<T> SampleTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new(probability: f64) -> Self {
    assert!(
      (0.0..=1.0).contains(&probability),
      "Probability must be between 0 and 1"
    );
    Self {
      probability,
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
