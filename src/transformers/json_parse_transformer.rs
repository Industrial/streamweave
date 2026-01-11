//! JSON parse transformer for parsing JSON strings into JSON values.
//!
//! This module provides [`JsonParseTransformer`], a transformer that parses JSON
//! strings from stream items into JSON values. It converts JSON-formatted strings
//! into `serde_json::Value` structures, making it ideal for processing JSON data
//! in pipelines. It implements the [`Transformer`] trait for use in StreamWeave
//! pipelines and graphs.
//!
//! # Overview
//!
//! [`JsonParseTransformer`] is useful for parsing JSON data in StreamWeave pipelines.
//! It processes JSON-formatted strings and converts them into structured JSON values,
//! making it ideal for deserializing JSON data for further processing.
//!
//! # Key Concepts
//!
//! - **JSON Parsing**: Parses JSON strings into `serde_json::Value` structures
//! - **String Input**: Takes JSON strings as input
//! - **Value Output**: Produces JSON values as output
//! - **Transformer Trait**: Implements `Transformer` for pipeline integration
//!
//! # Core Types
//!
//! - **[`JsonParseTransformer`]**: Transformer that parses JSON strings into values
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::JsonParseTransformer;
//!
//! // Create a JSON parse transformer
//! let transformer = JsonParseTransformer::new();
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::JsonParseTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a JSON parse transformer with error handling
//! let transformer = JsonParseTransformer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("json-parser".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Library Integration**: Uses `serde_json` for robust JSON parsing
//! - **Value Output**: Produces `serde_json::Value` for flexible JSON structure
//!   handling
//! - **Transformer Trait**: Implements `Transformer` for integration with
//!   pipeline system
//!
//! # Integration with StreamWeave
//!
//! [`JsonParseTransformer`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave pipeline or graph. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// A transformer that parses JSON strings from stream items.
///
/// Input: String (JSON string)
/// Output: `serde_json::Value` (parsed JSON)
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::JsonParseTransformer;
///
/// let transformer = JsonParseTransformer::new();
/// // Input: ["{\"name\": \"Alice\"}", "{\"name\": \"Bob\"}"]
/// // Output: [Value::Object(...), Value::Object(...)]
/// ```
pub struct JsonParseTransformer {
  /// Transformer configuration
  config: TransformerConfig<String>,
}

impl JsonParseTransformer {
  /// Creates a new `JsonParseTransformer`.
  pub fn new() -> Self {
    Self {
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

impl Default for JsonParseTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for JsonParseTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for JsonParseTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for JsonParseTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for JsonParseTransformer {
  type InputPorts = (String,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "json_parse_transformer".to_string());

    Box::pin(input.filter_map(move |item| {
      let component_name = component_name.clone();
      async move {
        match serde_json::from_str::<Value>(&item) {
          Ok(value) => Some(value),
          Err(e) => {
            tracing::warn!(
              component = %component_name,
              error = %e,
              input = %item,
              "Failed to parse JSON, skipping item"
            );
            None
          }
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
        .unwrap_or_else(|| "json_parse_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
