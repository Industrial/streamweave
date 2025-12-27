use streamweave::TransformerConfig;
use streamweave_error::ErrorStrategy;

/// A transformer that applies a function to each item in the stream.
///
/// This transformer takes each input item and applies a function to transform it
/// into an output item, creating a one-to-one mapping.
pub struct MapTransformer<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The function to apply to each input item.
  pub f: F,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<I>,
  /// Phantom data to track the input type parameter.
  pub _phantom_i: std::marker::PhantomData<I>,
  /// Phantom data to track the output type parameter.
  pub _phantom_o: std::marker::PhantomData<O>,
}

impl<F, I, O> MapTransformer<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `MapTransformer` with the given function.
  ///
  /// # Arguments
  ///
  /// * `f` - The function to apply to each input item.
  pub fn new(f: F) -> Self {
    Self {
      f,
      config: TransformerConfig::default(),
      _phantom_i: std::marker::PhantomData,
      _phantom_o: std::marker::PhantomData,
    }
  }

  /// Sets the error handling strategy for this transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<I>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this transformer.
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
