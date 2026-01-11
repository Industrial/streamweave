//! CSV parse transformer for parsing CSV strings into structured records.
//!
//! This module provides [`CsvParseTransformer`] and [`CsvParseConfig`], types for
//! parsing CSV strings from stream items into structured records in StreamWeave
//! pipelines. It converts CSV-formatted strings into JSON objects (one per row),
//! making it ideal for processing CSV data in pipelines. It implements the
//! [`Transformer`] trait for use in StreamWeave pipelines and graphs.
//!
//! # Overview
//!
//! [`CsvParseTransformer`] is useful for parsing CSV data in StreamWeave pipelines.
//! It processes CSV-formatted strings and converts them into structured JSON objects,
//! supporting configurable delimiters, headers, and trimming options.
//!
//! # Key Concepts
//!
//! - **CSV Parsing**: Parses CSV strings into structured JSON records
//! - **Header Support**: Supports CSV files with or without header rows
//! - **Configurable Delimiters**: Supports custom delimiter characters
//! - **Trimming**: Optional whitespace trimming from fields
//!
//! # Core Types
//!
//! - **[`CsvParseTransformer`]**: Transformer that parses CSV strings into records
//! - **[`CsvParseConfig`]**: Configuration for CSV parsing behavior
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::CsvParseTransformer;
//!
//! // Create a CSV parse transformer with default settings
//! let transformer = CsvParseTransformer::new();
//! ```
//!
//! ## With Configuration
//!
//! ```rust
//! use streamweave::transformers::CsvParseTransformer;
//!
//! // Create a CSV parse transformer with custom configuration
//! let transformer = CsvParseTransformer::new()
//!     .with_headers(true)
//!     .with_delimiter(b';')  // Use semicolon delimiter
//!     .with_trim(true);      // Trim whitespace
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::CsvParseTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a CSV parse transformer with error handling
//! let transformer = CsvParseTransformer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("csv-parser".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **CSV Library Integration**: Uses the `csv` crate for robust CSV parsing
//! - **JSON Output**: Produces JSON objects for flexible data structure handling
//! - **Configurable Options**: Supports headers, delimiters, and trimming for
//!   flexibility
//! - **Transformer Trait**: Implements `Transformer` for integration with
//!   pipeline system
//!
//! # Integration with StreamWeave
//!
//! [`CsvParseTransformer`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave pipeline or graph. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use csv::{ReaderBuilder, Trim};
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::io::Cursor;
use std::pin::Pin;

/// Configuration for CSV parsing.
#[derive(Debug, Clone)]
pub struct CsvParseConfig {
  /// Whether the CSV has a header row.
  pub has_headers: bool,
  /// The delimiter character (default: comma).
  pub delimiter: u8,
  /// Whether to trim whitespace from fields.
  pub trim: bool,
}

impl Default for CsvParseConfig {
  fn default() -> Self {
    Self {
      has_headers: true,
      delimiter: b',',
      trim: false,
    }
  }
}

/// A transformer that parses CSV strings from stream items.
///
/// Input: String (CSV string)
/// Output: `serde_json::Value` (array of objects, one per row)
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::CsvParseTransformer;
///
/// let transformer = CsvParseTransformer::new();
/// // Input: ["name,age\nAlice,30\nBob,25"]
/// // Output: [Value::Array([{"name": "Alice", "age": "30"}, {"name": "Bob", "age": "25"}])]
/// ```
pub struct CsvParseTransformer {
  /// CSV parsing configuration
  csv_config: CsvParseConfig,
  /// Transformer configuration
  config: TransformerConfig<String>,
}

impl CsvParseTransformer {
  /// Creates a new `CsvParseTransformer`.
  pub fn new() -> Self {
    Self {
      csv_config: CsvParseConfig::default(),
      config: TransformerConfig::default(),
    }
  }

  /// Sets whether the CSV has a header row.
  pub fn with_headers(mut self, has_headers: bool) -> Self {
    self.csv_config.has_headers = has_headers;
    self
  }

  /// Sets the delimiter character.
  pub fn with_delimiter(mut self, delimiter: u8) -> Self {
    self.csv_config.delimiter = delimiter;
    self
  }

  /// Sets whether to trim whitespace from fields.
  pub fn with_trim(mut self, trim: bool) -> Self {
    self.csv_config.trim = trim;
    self
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

impl Default for CsvParseTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for CsvParseTransformer {
  fn clone(&self) -> Self {
    Self {
      csv_config: self.csv_config.clone(),
      config: self.config.clone(),
    }
  }
}

impl Input for CsvParseTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for CsvParseTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for CsvParseTransformer {
  type InputPorts = (String,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let csv_config = self.csv_config.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "csv_parse_transformer".to_string());

    Box::pin(input.filter_map(move |item| {
      let csv_config = csv_config.clone();
      let component_name = component_name.clone();
      async move {
        let mut reader = ReaderBuilder::new()
          .has_headers(csv_config.has_headers)
          .delimiter(csv_config.delimiter)
          .trim(if csv_config.trim {
            Trim::All
          } else {
            Trim::None
          })
          .from_reader(Cursor::new(item.as_bytes()));

        let mut records = Vec::new();
        let headers = if csv_config.has_headers {
          reader
            .headers()
            .ok()
            .map(|h| h.iter().map(|s| s.to_string()).collect::<Vec<String>>())
        } else {
          None
        };

        for result in reader.records() {
          match result {
            Ok(record) => {
              let mut obj = serde_json::Map::new();
              if let Some(ref headers) = headers {
                for (i, field) in record.iter().enumerate() {
                  if i < headers.len() {
                    obj.insert(headers[i].clone(), Value::String(field.to_string()));
                  }
                }
              } else {
                // No headers - use column indices
                for (i, field) in record.iter().enumerate() {
                  obj.insert(format!("column_{}", i), Value::String(field.to_string()));
                }
              }
              records.push(Value::Object(obj));
            }
            Err(e) => {
              tracing::warn!(
                component = %component_name,
                error = %e,
                "Failed to parse CSV record, skipping"
              );
            }
          }
        }

        if records.is_empty() {
          None
        } else {
          Some(Value::Array(records))
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
        .unwrap_or_else(|| "csv_parse_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
