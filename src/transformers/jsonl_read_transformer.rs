//! # JSONL Read Transformer
//!
//! Transformer that reads JSON Lines (JSONL) files from input file paths and
//! deserializes them line by line. JSONL is a text format where each line is
//! a valid JSON value, making it ideal for streaming large datasets.
//!
//! ## Overview
//!
//! The JSONL Read Transformer provides:
//!
//! - **JSONL File Reading**: Reads and parses JSONL files line by line
//! - **Type-Safe Deserialization**: Deserializes each line into strongly-typed Rust types
//! - **Streaming Format**: Processes one line at a time for memory efficiency
//! - **Error Handling**: Configurable error strategies for parse failures
//!
//! ## Input/Output
//!
//! - **Input**: `Message<String>` - File paths to JSONL files
//! - **Output**: `Message<T>` - Deserialized JSONL objects (type depends on T)
//!
//! ## JSONL Format
//!
//! JSON Lines is a text format where each line is a valid JSON value:
//!
//! ```jsonl
//! {"id": 1, "message": "hello"}
//! {"id": 2, "message": "world"}
//! ```
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::transformers::JsonlReadTransformer;
//! use serde::Deserialize;
//!
//! #[derive(Debug, Deserialize, Clone)]
//! struct Event {
//!     id: u32,
//!     message: String,
//! }
//!
//! let transformer = JsonlReadTransformer::<Event>::new();
//! // Input: ["events1.jsonl", "events2.jsonl"]
//! // Output: [Event { id: 1, message: "..." }, Event { id: 2, message: "..." }, ...]
//! ```

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use std::pin::Pin;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::error;

/// A transformer that reads JSONL files from input paths and deserializes them line by line.
///
/// JSON Lines is a text format where each line is a valid JSON value.
/// This transformer reads each line from a file and deserializes it as the specified type.
///
/// Input: String (file path)
/// Output: T (deserialized JSONL object)
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::JsonlReadTransformer;
/// use serde::Deserialize;
///
/// #[derive(Debug, Deserialize, Clone)]
/// struct Event {
///     id: u32,
///     message: String,
/// }
///
/// let transformer = JsonlReadTransformer::<Event>::new();
/// // Input: ["events1.jsonl", "events2.jsonl"]
/// // Output: [Event { id: 1, message: "..." }, Event { id: 2, message: "..." }, ...]
/// ```
pub struct JsonlReadTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// Transformer configuration.
  pub config: TransformerConfig<String>,
  /// Phantom data for the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> JsonlReadTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// Creates a new `JsonlReadTransformer`.
  #[must_use]
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    }
  }

  /// Sets the error strategy for the transformer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the transformer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<T> Default for JsonlReadTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> Clone for JsonlReadTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      _phantom: PhantomData,
    }
  }
}

impl<T> Input for JsonlReadTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

impl<T> Output for JsonlReadTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl<T> Transformer for JsonlReadTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type InputPorts = (String,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "jsonl_read_transformer".to_string());
    let error_strategy = self.config.error_strategy.clone();

    Box::pin(input.flat_map(move |path| {
      let component_name_clone = component_name.clone();
      let error_strategy_clone = error_strategy.clone();

      // Use async_stream to handle the async file reading and parsing
      async_stream::stream! {
        match File::open(&path).await {
          Ok(file) => {
            let reader = BufReader::new(file);
            let mut lines = reader.lines();

            loop {
              match lines.next_line().await {
                Ok(Some(line)) => {
                  match serde_json::from_str::<T>(&line) {
                    Ok(item) => yield item,
                    Err(e) => {
                      let stream_error = StreamError::new(
                        Box::new(e),
                        ErrorContext {
                          timestamp: chrono::Utc::now(),
                          item: Some(path.clone()),
                          component_name: component_name_clone.clone(),
                          component_type: std::any::type_name::<JsonlReadTransformer<T>>().to_string(),
                        },
                        ComponentInfo {
                          name: component_name_clone.clone(),
                          type_name: std::any::type_name::<JsonlReadTransformer<T>>().to_string(),
                        },
                      );
                      match handle_error_strategy(&error_strategy_clone, &stream_error) {
                        ErrorAction::Stop => {
                          error!(
                            component = %component_name_clone,
                            path = %path,
                            error = %stream_error,
                            "Stopping due to deserialization error"
                          );
                          break;
                        }
                        ErrorAction::Skip => {
                          // Continue to next line
                        }
                        ErrorAction::Retry => {
                          // Retry not directly supported for line deserialization
                        }
                      }
                    }
                  }
                }
                Ok(None) => break,
                Err(e) => {
                  let stream_error = StreamError::new(
                    Box::new(e),
                    ErrorContext {
                      timestamp: chrono::Utc::now(),
                      item: Some(path.clone()),
                      component_name: component_name_clone.clone(),
                      component_type: std::any::type_name::<JsonlReadTransformer<T>>().to_string(),
                    },
                    ComponentInfo {
                      name: component_name_clone.clone(),
                      type_name: std::any::type_name::<JsonlReadTransformer<T>>().to_string(),
                    },
                  );
                  match handle_error_strategy(&error_strategy_clone, &stream_error) {
                    ErrorAction::Stop => {
                      error!(
                        component = %component_name_clone,
                        path = %path,
                        error = %stream_error,
                        "Stopping due to line read error"
                      );
                      break;
                    }
                    ErrorAction::Skip => {
                      // Continue to next line
                    }
                    ErrorAction::Retry => {
                      // Retry not directly supported for line read errors
                    }
                  }
                }
              }
            }
          }
          Err(e) => {
            error!(
              component = %component_name_clone,
              path = %path,
              error = %e,
              "Failed to open JSONL file"
            );
            let stream_error = StreamError::new(
              Box::new(e),
              ErrorContext {
                timestamp: chrono::Utc::now(),
                item: Some(path.clone()),
                component_name: component_name_clone.clone(),
                component_type: std::any::type_name::<JsonlReadTransformer<T>>().to_string(),
              },
              ComponentInfo {
                name: component_name_clone.clone(),
                type_name: std::any::type_name::<JsonlReadTransformer<T>>().to_string(),
              },
            );
            match handle_error_strategy(&error_strategy_clone, &stream_error) {
              ErrorAction::Stop => {
                // Stop processing this file
              }
              ErrorAction::Skip => {
                // Skip this file and continue
              }
              ErrorAction::Retry => {
                // Retry not directly supported for file open errors
              }
            }
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
    match self.config.error_strategy() {
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
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "jsonl_read_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "jsonl_read_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

/// Helper function to handle error strategy
pub(crate) fn handle_error_strategy<T>(
  strategy: &ErrorStrategy<T>,
  error: &StreamError<T>,
) -> ErrorAction
where
  T: std::fmt::Debug + Clone + Send + Sync,
{
  match strategy {
    ErrorStrategy::Stop => ErrorAction::Stop,
    ErrorStrategy::Skip => ErrorAction::Skip,
    ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
    ErrorStrategy::Custom(handler) => handler(error),
    _ => ErrorAction::Stop,
  }
}
