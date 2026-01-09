//! String pad transformer for StreamWeave
//!
//! Pads strings to a specified length with a padding character.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// Padding direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadDirection {
  /// Pad on the left
  Left,
  /// Pad on the right
  Right,
}

/// A transformer that pads strings to a specified length.
///
/// Pads strings with a padding character to reach the target length.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::{StringPadTransformer, PadDirection};
///
/// let transformer = StringPadTransformer::new(10, ' ', PadDirection::Right);
/// // Input: ["hello"]
/// // Output: ["hello     "]
/// ```
pub struct StringPadTransformer {
  /// Target length
  length: usize,
  /// Padding character
  pad_char: char,
  /// Padding direction
  direction: PadDirection,
  /// Configuration for the transformer
  config: TransformerConfig<String>,
}

impl StringPadTransformer {
  /// Creates a new `StringPadTransformer`.
  ///
  /// # Arguments
  ///
  /// * `length` - Target length for the padded string.
  /// * `pad_char` - Character to use for padding.
  /// * `direction` - Direction to pad (Left or Right).
  pub fn new(length: usize, pad_char: char, direction: PadDirection) -> Self {
    Self {
      length,
      pad_char,
      direction,
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

impl Clone for StringPadTransformer {
  fn clone(&self) -> Self {
    Self {
      length: self.length,
      pad_char: self.pad_char,
      direction: self.direction,
      config: self.config.clone(),
    }
  }
}

impl Input for StringPadTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringPadTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for StringPadTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let length = self.length;
    let pad_char = self.pad_char;
    let direction = self.direction;
    Box::pin(input.map(move |s| {
      let current_len = s.len();
      if current_len >= length {
        s
      } else {
        let pad_len = length - current_len;
        let padding: String = (0..pad_len).map(|_| pad_char).collect();
        match direction {
          PadDirection::Left => format!("{}{}", padding, s),
          PadDirection::Right => format!("{}{}", s, padding),
        }
      }
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
        .unwrap_or_else(|| "string_pad_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
