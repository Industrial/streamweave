use crate::error::{ComponentInfo, ErrorContext, ErrorStrategy, StreamError};
use crate::transformer::TransformerConfig;
use std::marker::PhantomData;

pub struct BatchTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub size: usize,
  pub config: TransformerConfig<T>,
  pub _phantom: PhantomData<T>,
}

impl<T> BatchTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
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

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}
