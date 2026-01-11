//! CSV stringify transformer for converting JSON data to CSV strings.
//!
//! This module provides [`CsvStringifyTransformer`], a transformer that converts
//! JSON objects or arrays of objects into CSV-formatted strings. It's the inverse
//! of `CsvParseTransformer`, useful for exporting structured data to CSV format
//! for storage, transmission, or further processing.
//!
//! # Overview
//!
//! [`CsvStringifyTransformer`] takes JSON objects or arrays of objects and converts
//! them into CSV strings. It supports configurable delimiters, optional header rows,
//! and handles various JSON value types (strings, numbers, booleans, null).
//!
//! # Key Concepts
//!
//! - **CSV Generation**: Converts JSON objects/arrays into CSV-formatted strings
//! - **Header Support**: Optionally writes header row with field names
//! - **Configurable Delimiters**: Supports custom delimiter characters (comma, semicolon, etc.)
//! - **Type Handling**: Converts JSON values (strings, numbers, booleans, null) to CSV
//! - **Array Processing**: Handles single objects or arrays of objects
//!
//! # Core Types
//!
//! - **[`CsvStringifyTransformer`]**: Transformer that converts JSON to CSV strings
//! - **[`CsvStringifyConfig`]**: Configuration for CSV stringification behavior
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::CsvStringifyTransformer;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a CSV stringify transformer
//! let transformer = CsvStringifyTransformer::new();
//!
//! // Input: [json!([{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}])]
//! // Output: ["name,age\nAlice,30\nBob,25"]
//! # Ok(())
//! # }
//! ```
//!
//! ## With Configuration
//!
//! ```rust
//! use streamweave::transformers::CsvStringifyTransformer;
//!
//! // Create a transformer with custom delimiter and no headers
//! let transformer = CsvStringifyTransformer::new()
//!     .with_headers(false)      // Don't write header row
//!     .with_delimiter(b';');    // Use semicolon delimiter
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::CsvStringifyTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a transformer with error handling
//! let transformer = CsvStringifyTransformer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("csv-exporter".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **CSV Library Integration**: Uses the `csv` crate for robust CSV generation
//! - **JSON Input**: Accepts JSON Value types for flexible data structures
//! - **Header Detection**: Automatically detects headers from first object's keys
//! - **Type Conversion**: Converts JSON values to strings for CSV output
//! - **Array Handling**: Supports both single objects and arrays of objects
//!
//! # Integration with StreamWeave
//!
//! [`CsvStringifyTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline or graph. It supports the standard error handling
//! strategies and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use csv::WriterBuilder;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// Configuration for CSV stringification.
#[derive(Debug, Clone)]
pub struct CsvStringifyConfig {
  /// Whether to write a header row.
  pub write_headers: bool,
  /// The delimiter character (default: comma).
  pub delimiter: u8,
}

impl Default for CsvStringifyConfig {
  fn default() -> Self {
    Self {
      write_headers: true,
      delimiter: b',',
    }
  }
}

/// A transformer that stringifies JSON values to CSV from stream items.
///
/// Input: `serde_json::Value` (object or array of objects)
/// Output: String (CSV string)
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::CsvStringifyTransformer;
/// use serde_json::json;
///
/// let transformer = CsvStringifyTransformer::new();
/// // Input: [json!([{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}])]
/// // Output: ["name,age\nAlice,30\nBob,25"]
/// ```
pub struct CsvStringifyTransformer {
  /// CSV stringification configuration
  csv_config: CsvStringifyConfig,
  /// Transformer configuration
  config: TransformerConfig<Value>,
}

impl CsvStringifyTransformer {
  /// Creates a new `CsvStringifyTransformer`.
  pub fn new() -> Self {
    Self {
      csv_config: CsvStringifyConfig::default(),
      config: TransformerConfig::default(),
    }
  }

  /// Sets whether to write a header row.
  pub fn with_headers(mut self, write_headers: bool) -> Self {
    self.csv_config.write_headers = write_headers;
    self
  }

  /// Sets the delimiter character.
  pub fn with_delimiter(mut self, delimiter: u8) -> Self {
    self.csv_config.delimiter = delimiter;
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

impl Default for CsvStringifyTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for CsvStringifyTransformer {
  fn clone(&self) -> Self {
    Self {
      csv_config: self.csv_config.clone(),
      config: self.config.clone(),
    }
  }
}

impl Input for CsvStringifyTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for CsvStringifyTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for CsvStringifyTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let csv_config = self.csv_config.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "csv_stringify_transformer".to_string());

    Box::pin(input.filter_map(move |item| {
      let csv_config = csv_config.clone();
      let component_name = component_name.clone();
      async move {
        // Handle array of objects or single object
        let records: Vec<&Value> = match &item {
          Value::Array(arr) => arr.iter().collect(),
          Value::Object(_) => vec![&item],
          _ => {
            tracing::warn!(
              component = %component_name,
              "CSV stringify requires object or array of objects, got: {:?}",
              item
            );
            return None;
          }
        };

        if records.is_empty() {
          return None;
        }

        // Get headers from first object
        let headers = match records[0] {
          Value::Object(obj) => obj.keys().cloned().collect::<Vec<String>>(),
          _ => {
            tracing::warn!(
              component = %component_name,
              "CSV stringify requires objects, got: {:?}",
              records[0]
            );
            return None;
          }
        };

        // Build CSV
        let mut wtr = WriterBuilder::new()
          .has_headers(csv_config.write_headers)
          .delimiter(csv_config.delimiter)
          .from_writer(Vec::new());

        // Write headers if needed
        if csv_config.write_headers && wtr.write_record(&headers).is_err() {
          return None;
        }

        // Write records
        for record in records {
          if let Value::Object(obj) = record {
            let values: Vec<String> = headers
              .iter()
              .map(|h| {
                obj
                  .get(h)
                  .map(|v| match v {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Null => String::new(),
                    _ => serde_json::to_string(v).unwrap_or_default(),
                  })
                  .unwrap_or_default()
              })
              .collect();
            if wtr.write_record(&values).is_err() {
              return None;
            }
          }
        }

        wtr.flush().ok()?;
        let data = wtr.into_inner().ok()?;
        String::from_utf8(data).ok()
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
        .unwrap_or_else(|| "csv_stringify_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
