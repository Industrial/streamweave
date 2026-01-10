//! String predicate transformer for StreamWeave
//!
//! Filters strings based on predicates (starts_with, ends_with, contains).

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// Predicate type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredicateType {
  /// Check if string starts with substring
  StartsWith,
  /// Check if string ends with substring
  EndsWith,
  /// Check if string contains substring
  Contains,
}

/// A transformer that filters strings based on predicates.
///
/// Filters strings that match the specified predicate (starts_with, ends_with, or contains).
///
/// # Example
///
/// ```rust
/// use crate::transformers::{StringPredicateTransformer, PredicateType};
///
/// let transformer = StringPredicateTransformer::new("hello", PredicateType::StartsWith);
/// // Input: ["hello world", "hi there", "hello again"]
/// // Output: ["hello world", "hello again"]
/// ```
pub struct StringPredicateTransformer {
  /// Substring to check
  substring: String,
  /// Predicate type
  predicate_type: PredicateType,
  /// Configuration for the transformer
  config: TransformerConfig<String>,
}

impl StringPredicateTransformer {
  /// Creates a new `StringPredicateTransformer`.
  ///
  /// # Arguments
  ///
  /// * `substring` - The substring to check for.
  /// * `predicate_type` - The type of predicate (StartsWith, EndsWith, or Contains).
  pub fn new(substring: impl Into<String>, predicate_type: PredicateType) -> Self {
    Self {
      substring: substring.into(),
      predicate_type,
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

impl Clone for StringPredicateTransformer {
  fn clone(&self) -> Self {
    Self {
      substring: self.substring.clone(),
      predicate_type: self.predicate_type,
      config: self.config.clone(),
    }
  }
}

impl Input for StringPredicateTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringPredicateTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for StringPredicateTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let substring = self.substring.clone();
    let predicate_type = self.predicate_type;
    Box::pin(input.filter(move |s| {
      let substring = substring.clone();
      let matches = match predicate_type {
        PredicateType::StartsWith => s.starts_with(&substring),
        PredicateType::EndsWith => s.ends_with(&substring),
        PredicateType::Contains => s.contains(&substring),
      };
      futures::future::ready(matches)
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
        .unwrap_or_else(|| "string_predicate_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
