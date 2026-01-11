//! JSON stringify transformer for converting JSON values into JSON strings.
//!
//! This module provides [`JsonStringifyTransformer`] and [`JsonStringifyConfig`],
//! types for converting JSON values from stream items into JSON strings in
//! StreamWeave pipelines. It serializes `serde_json::Value` structures into
//! JSON-formatted strings, with optional pretty-printing. It implements the
//! [`Transformer`] trait for use in StreamWeave pipelines and graphs.
//!
//! # Overview
//!
//! [`JsonStringifyTransformer`] is useful for serializing JSON data in StreamWeave
//! pipelines. It processes JSON values and converts them into JSON-formatted strings,
//! making it ideal for serializing JSON data for output or transmission.
//!
//! # Key Concepts
//!
//! - **JSON Serialization**: Converts `serde_json::Value` structures into JSON strings
//! - **Pretty Printing**: Optional pretty-printing for human-readable JSON
//! - **Value Input**: Takes JSON values as input
//! - **String Output**: Produces JSON strings as output
//!
//! # Core Types
//!
//! - **[`JsonStringifyTransformer`]**: Transformer that converts JSON values to strings
//! - **[`JsonStringifyConfig`]**: Configuration for JSON stringification behavior
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::JsonStringifyTransformer;
//!
//! // Create a JSON stringify transformer
//! let transformer = JsonStringifyTransformer::new();
//! ```
//!
//! ## With Pretty Printing
//!
//! ```rust
//! use streamweave::transformers::JsonStringifyTransformer;
//!
//! // Create a JSON stringify transformer with pretty-printing
//! let transformer = JsonStringifyTransformer::new()
//!     .with_pretty(true);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::JsonStringifyTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a JSON stringify transformer with error handling
//! let transformer = JsonStringifyTransformer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("json-stringifier".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Library Integration**: Uses `serde_json` for robust JSON serialization
//! - **Pretty Printing**: Optional formatting for human-readable output
//! - **Value Input**: Works with `serde_json::Value` for flexible JSON structure
//!   handling
//! - **Transformer Trait**: Implements `Transformer` for integration with
//!   pipeline system
//!
//! # Integration with StreamWeave
//!
//! [`JsonStringifyTransformer`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave pipeline or graph. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// Configuration for JSON stringification.
#[derive(Debug, Clone, Default)]
pub struct JsonStringifyConfig {
  /// Whether to pretty-print JSON.
  pub pretty: bool,
}

/// A transformer that stringifies JSON values from stream items.
///
/// Input: `serde_json::Value` (JSON value)
/// Output: String (JSON string)
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::JsonStringifyTransformer;
/// use serde_json::json;
///
/// let transformer = JsonStringifyTransformer::new();
/// // Input: [json!({"name": "Alice"}), json!({"name": "Bob"})]
/// // Output: ["{\"name\":\"Alice\"}", "{\"name\":\"Bob\"}"]
/// ```
pub struct JsonStringifyTransformer {
  /// JSON stringification configuration
  json_config: JsonStringifyConfig,
  /// Transformer configuration
  config: TransformerConfig<Value>,
}

impl JsonStringifyTransformer {
  /// Creates a new `JsonStringifyTransformer`.
  pub fn new() -> Self {
    Self {
      json_config: JsonStringifyConfig::default(),
      config: TransformerConfig::default(),
    }
  }

  /// Sets whether to pretty-print JSON.
  pub fn with_pretty(mut self, pretty: bool) -> Self {
    self.json_config.pretty = pretty;
    self
  }

  /// Sets the error handling strategy for this transformer.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Value>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Default for JsonStringifyTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for JsonStringifyTransformer {
  fn clone(&self) -> Self {
    Self {
      json_config: self.json_config.clone(),
      config: self.config.clone(),
    }
  }
}

impl Input for JsonStringifyTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for JsonStringifyTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for JsonStringifyTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let pretty = self.json_config.pretty;

    Box::pin(input.filter_map(move |item| {
      let pretty = pretty;
      async move {
        if pretty {
          serde_json::to_string_pretty(&item).ok()
        } else {
          serde_json::to_string(&item).ok()
        }
      }
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Value>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Value> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Value> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Value>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Value>) -> ErrorContext<Value> {
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
        .unwrap_or_else(|| "json_stringify_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
