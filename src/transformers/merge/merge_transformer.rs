use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use futures::Stream;
use std::marker::PhantomData;
use std::pin::Pin;

/// A transformer that merges multiple streams into a single stream.
///
/// This transformer combines items from multiple input streams into a single output stream,
/// interleaving items as they become available.
pub struct MergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Phantom data to track the type parameter.
  pub _phantom: PhantomData<T>,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// The streams to merge together.
  pub streams: Vec<Pin<Box<dyn Stream<Item = T> + Send>>>,
}

impl<T> Clone for MergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      _phantom: self._phantom,
      config: self.config.clone(),
      streams: Vec::new(), // Streams can't be cloned, so start with empty
    }
  }
}

impl<T> MergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `MergeTransformer` with no streams.
  ///
  /// Use `add_stream` to add streams to merge.
  pub fn new() -> Self {
    Self {
      _phantom: PhantomData,
      config: TransformerConfig::default(),
      streams: Vec::new(),
    }
  }

  /// Sets the error handling strategy for this transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
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

  /// Adds a stream to be merged.
  ///
  /// # Arguments
  ///
  /// * `stream` - The stream to add to the merge operation.
  pub fn add_stream(&mut self, stream: Pin<Box<dyn Stream<Item = T> + Send>>) {
    self.streams.push(stream);
  }
}

impl<T> Default for MergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}
