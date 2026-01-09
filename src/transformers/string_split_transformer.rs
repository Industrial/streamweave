//! String split transformer for StreamWeave
//!
//! Splits strings by a delimiter, producing a stream of substrings.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::stream;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// A transformer that splits strings by a delimiter.
///
/// Takes each input string and splits it by the specified delimiter,
/// producing a stream of substrings.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::StringSplitTransformer;
///
/// let transformer = StringSplitTransformer::new(",");
/// // Input: ["a,b,c", "x,y"]
/// // Output: ["a", "b", "c", "x", "y"]
/// ```
pub struct StringSplitTransformer {
  /// Delimiter to split by
  delimiter: String,
  /// Configuration for the transformer
  config: TransformerConfig<String>,
}

impl StringSplitTransformer {
  /// Creates a new `StringSplitTransformer` with the specified delimiter.
  ///
  /// # Arguments
  ///
  /// * `delimiter` - The delimiter to split by.
  pub fn new(delimiter: impl Into<String>) -> Self {
    Self {
      delimiter: delimiter.into(),
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this transformer.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Clone for StringSplitTransformer {
  fn clone(&self) -> Self {
    Self {
      delimiter: self.delimiter.clone(),
      config: self.config.clone(),
    }
  }
}

impl Input for StringSplitTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringSplitTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for StringSplitTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let delimiter = self.delimiter.clone();
    Box::pin(input.flat_map(move |s| {
      let parts: Vec<String> = s.split(&delimiter).map(|part| part.to_string()).collect();
      stream::iter(parts)
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<String> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "string_split_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
