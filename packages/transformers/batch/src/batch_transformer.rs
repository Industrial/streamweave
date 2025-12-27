use std::marker::PhantomData;
use streamweave::TransformerConfig;
use streamweave_error::{ComponentInfo, ErrorContext, ErrorStrategy, StreamError};

/// A transformer that groups items in a stream into batches of a specified size.
///
/// This transformer collects incoming items until the specified `size` is reached,
/// then emits them as a `Vec<T>`.
pub struct BatchTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The number of items to include in each batch.
  pub size: usize,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the input type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> BatchTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `BatchTransformer` with the given batch size.
  ///
  /// # Arguments
  ///
  /// * `size` - The number of items to include in each batch. Must be greater than zero.
  ///
  /// # Returns
  ///
  /// A `Result` containing the `BatchTransformer` if `size` is valid, or an error if `size` is zero.
  pub fn new(size: usize) -> Result<Self, Box<StreamError<T>>> {
    if size == 0 {
      return Err(Box::new(StreamError::new(
        Box::new(std::io::Error::new(
          std::io::ErrorKind::InvalidInput,
          "Batch size must be greater than zero",
        )),
        ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          component_name: "batch_transformer".to_string(),
          component_type: std::any::type_name::<Self>().to_string(),
        },
        ComponentInfo {
          name: "batch_transformer".to_string(),
          type_name: std::any::type_name::<Self>().to_string(),
        },
      )));
    }
    Ok(Self {
      size,
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    })
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
}
