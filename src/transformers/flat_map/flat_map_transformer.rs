use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use std::marker::PhantomData;

/// A transformer that applies a function to each item and flattens the result.
///
/// This transformer takes each input item, applies a function that returns a vector,
/// and then flattens all the resulting vectors into a single output stream.
pub struct FlatMapTransformer<F, I, O>
where
  F: Fn(I) -> Vec<O> + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The function to apply to each input item.
  pub f: F,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<I>,
  /// Phantom data to track the input type parameter.
  pub _phantom_i: PhantomData<I>,
  /// Phantom data to track the output type parameter.
  pub _phantom_o: PhantomData<O>,
}

impl<F, I, O> FlatMapTransformer<F, I, O>
where
  F: Fn(I) -> Vec<O> + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `FlatMapTransformer` with the given function.
  ///
  /// # Arguments
  ///
  /// * `f` - The function to apply to each input item, which should return a vector.
  pub fn new(f: F) -> Self {
    Self {
      f,
      config: TransformerConfig::default(),
      _phantom_i: PhantomData,
      _phantom_o: PhantomData,
    }
  }

  /// Sets the error handling strategy for this transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<I>) -> Self {
    self.config = self.config.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this transformer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config = self.config.with_name(name);
    self
  }
}
