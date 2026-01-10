//! JSON read transformer for StreamWeave
//!
//! Reads JSON files from input paths and deserializes them.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::producers::JsonReadConfig;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use std::pin::Pin;
use tokio::fs::read_to_string;
use tracing::error;

/// A transformer that reads JSON files from input paths and deserializes them.
///
/// Input: String (file path)
/// Output: T (deserialized JSON object)
///
/// This transformer can handle:
/// - JSON objects: yields the object as a single item
/// - JSON arrays: can yield each element as a separate item (if `array_as_stream` is true)
/// - JSON primitives: yields the value as a single item
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::JsonReadTransformer;
/// use serde::Deserialize;
///
/// #[derive(Debug, Deserialize, Clone)]
/// struct Event {
///     id: u32,
///     message: String,
/// }
///
/// let transformer = JsonReadTransformer::<Event>::new();
/// // Input: ["events1.json", "events2.json"]
/// // Output: [Event { id: 1, message: "..." }, Event { id: 2, message: "..." }, ...]
/// ```
pub struct JsonReadTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// Transformer configuration.
  pub config: TransformerConfig<String>,
  /// JSON-specific configuration.
  pub json_config: JsonReadConfig,
  /// Phantom data for the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> JsonReadTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// Creates a new `JsonReadTransformer`.
  #[must_use]
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
      json_config: JsonReadConfig::default(),
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

  /// Sets whether to treat JSON arrays as a stream of items.
  ///
  /// When true, if the JSON file contains an array, each element will be yielded as a separate item.
  /// When false, the entire array will be yielded as a single item.
  #[must_use]
  pub fn with_array_as_stream(mut self, array_as_stream: bool) -> Self {
    self.json_config = self.json_config.with_array_as_stream(array_as_stream);
    self
  }
}

impl<T> Default for JsonReadTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> Clone for JsonReadTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      json_config: self.json_config.clone(),
      _phantom: PhantomData,
    }
  }
}

impl<T> Input for JsonReadTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

impl<T> Output for JsonReadTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl<T> Transformer for JsonReadTransformer<T>
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
      .unwrap_or_else(|| "json_read_transformer".to_string());
    let error_strategy = self.config.error_strategy.clone();
    let array_as_stream = self.json_config.array_as_stream;

    Box::pin(input.flat_map(move |path| {
      let component_name_clone = component_name.clone();
      let error_strategy_clone = error_strategy.clone();

      // Use async_stream to handle the async file reading and parsing
      async_stream::stream! {
        // Read the entire file
        let content = match read_to_string(&path).await {
          Ok(content) => content,
          Err(e) => {
            error!(
              component = %component_name_clone,
              path = %path,
              error = %e,
              "Failed to read JSON file"
            );
            let stream_error = StreamError::new(
              Box::new(e),
              ErrorContext {
                timestamp: chrono::Utc::now(),
                item: Some(path.clone()),
                component_name: component_name_clone.clone(),
                component_type: std::any::type_name::<JsonReadTransformer<T>>().to_string(),
              },
              ComponentInfo {
                name: component_name_clone.clone(),
                type_name: std::any::type_name::<JsonReadTransformer<T>>().to_string(),
              },
            );
            match handle_error_strategy(&error_strategy_clone, &stream_error) {
              ErrorAction::Stop => {
                // Stop processing this file
                return;
              }
              ErrorAction::Skip => {
                // Skip this file and continue
                return;
              }
              ErrorAction::Retry => {
                // Retry not directly supported for file read errors
                return;
              }
            }
          }
        };

        // Try to parse as JSON value first to detect if it's an array
        match serde_json::from_str::<serde_json::Value>(&content) {
          Ok(json_value) => {
            // If it's an array and we want to stream elements
            if array_as_stream && json_value.is_array() {
              if let Some(array) = json_value.as_array() {
                for (idx, item) in array.iter().enumerate() {
                  match serde_json::from_value::<T>(item.clone()) {
                    Ok(deserialized) => yield deserialized,
                    Err(e) => {
                      let stream_error = StreamError::new(
                        Box::new(e),
                        ErrorContext {
                          timestamp: chrono::Utc::now(),
                          item: Some(path.clone()),
                          component_name: component_name_clone.clone(),
                          component_type: std::any::type_name::<JsonReadTransformer<T>>().to_string(),
                        },
                        ComponentInfo {
                          name: component_name_clone.clone(),
                          type_name: std::any::type_name::<JsonReadTransformer<T>>().to_string(),
                        },
                      );
                      match handle_error_strategy(&error_strategy_clone, &stream_error) {
                        ErrorAction::Stop => {
                          error!(
                            component = %component_name_clone,
                            path = %path,
                            index = idx,
                            error = %stream_error,
                            "Stopping due to deserialization error at array index"
                          );
                          return;
                        }
                        ErrorAction::Skip => {
                          // Continue to next array element
                        }
                        ErrorAction::Retry => {
                          // Retry not directly supported for array element deserialization
                        }
                      }
                    }
                  }
                }
              }
            } else {
              // Not an array, or array_as_stream is false - deserialize as T
              match serde_json::from_str::<T>(&content) {
                Ok(item) => yield item,
                Err(e) => {
                  let stream_error = StreamError::new(
                    Box::new(e),
                    ErrorContext {
                      timestamp: chrono::Utc::now(),
                      item: Some(path.clone()),
                      component_name: component_name_clone.clone(),
                      component_type: std::any::type_name::<JsonReadTransformer<T>>().to_string(),
                    },
                    ComponentInfo {
                      name: component_name_clone.clone(),
                      type_name: std::any::type_name::<JsonReadTransformer<T>>().to_string(),
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
                      return;
                    }
                    ErrorAction::Skip => {
                      // Skip this file
                    }
                    ErrorAction::Retry => {
                      // Retry not directly supported for deserialization errors
                    }
                  }
                }
              }
            }
          }
          Err(e) => {
            let stream_error = StreamError::new(
              Box::new(e),
              ErrorContext {
                timestamp: chrono::Utc::now(),
                item: Some(path.clone()),
                component_name: component_name_clone.clone(),
                component_type: std::any::type_name::<JsonReadTransformer<T>>().to_string(),
              },
              ComponentInfo {
                name: component_name_clone.clone(),
                type_name: std::any::type_name::<JsonReadTransformer<T>>().to_string(),
              },
            );
            match handle_error_strategy(&error_strategy_clone, &stream_error) {
              ErrorAction::Stop => {
                error!(
                  component = %component_name_clone,
                  path = %path,
                  error = %stream_error,
                  "Stopping due to JSON parse error"
                );
                return;
              }
              ErrorAction::Skip => {
                // Skip this file
              }
              ErrorAction::Retry => {
                // Retry not directly supported for JSON parse errors
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
        .unwrap_or_else(|| "json_read_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "json_read_transformer".to_string()),
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
