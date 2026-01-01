use async_trait::async_trait;
use futures::Stream;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use std::pin::Pin;
use streamweave::{Output, Producer, ProducerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tokio::fs::read_to_string;
use tracing::{error, warn};

/// Configuration for JSON reading behavior.
#[derive(Debug, Clone)]
pub struct JsonReadConfig {
  /// Whether to treat JSON arrays as a stream of items (true) or as a single item (false).
  /// When true, if the JSON file contains an array, each element will be yielded as a separate item.
  /// When false, the entire array will be yielded as a single item.
  pub array_as_stream: bool,
}

impl Default for JsonReadConfig {
  fn default() -> Self {
    Self {
      array_as_stream: true,
    }
  }
}

impl JsonReadConfig {
  /// Sets whether to treat JSON arrays as a stream of items.
  #[must_use]
  pub fn with_array_as_stream(mut self, array_as_stream: bool) -> Self {
    self.array_as_stream = array_as_stream;
    self
  }
}

/// A producer that reads JSON files.
///
/// Unlike JSONL (JSON Lines), this producer reads complete JSON documents.
/// It can handle:
/// - JSON objects: yields the object as a single item
/// - JSON arrays: can yield each element as a separate item (if `array_as_stream` is true)
/// - JSON primitives: yields the value as a single item
///
/// # Example
///
/// ```ignore
/// use streamweave_json_producer::JsonProducer;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Event {
///     id: u32,
///     message: String,
/// }
///
/// // Read a JSON array and stream each element
/// let producer = JsonProducer::<Event>::new("events.json");
///
/// // Or read the entire array as a single item
/// let producer = JsonProducer::<Vec<Event>>::new("events.json")
///     .with_array_as_stream(false);
/// ```
pub struct JsonProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// The path to the JSON file to read from.
  pub path: String,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<T>,
  /// JSON-specific configuration.
  pub json_config: JsonReadConfig,
  /// Phantom data to track the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> JsonProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// Creates a new JSON producer for the specified file path.
  #[must_use]
  pub fn new(path: impl Into<String>) -> Self {
    Self {
      path: path.into(),
      config: ProducerConfig::default(),
      json_config: JsonReadConfig::default(),
      _phantom: PhantomData,
    }
  }

  /// Sets the error strategy for the producer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the producer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Sets whether to treat JSON arrays as a stream of items.
  #[must_use]
  pub fn with_array_as_stream(mut self, array_as_stream: bool) -> Self {
    self.json_config.array_as_stream = array_as_stream;
    self
  }

  /// Returns the file path.
  #[must_use]
  pub fn path(&self) -> &str {
    &self.path
  }
}

impl<T> Clone for JsonProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  fn clone(&self) -> Self {
    Self {
      path: self.path.clone(),
      config: self.config.clone(),
      json_config: self.json_config.clone(),
      _phantom: PhantomData,
    }
  }
}

// Trait implementations for JsonProducer

impl<T> Output for JsonProducer<T>
where
  T: DeserializeOwned + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl<T> Producer for JsonProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type OutputPorts = (T,);

  /// Produces a stream of deserialized JSON objects from a JSON file.
  ///
  /// # Behavior
  ///
  /// - If the JSON file contains an array and `array_as_stream` is true, each element is yielded separately.
  /// - If the JSON file contains an array and `array_as_stream` is false, the entire array is yielded as a single item.
  /// - If the JSON file contains an object or primitive, it is yielded as a single item.
  ///
  /// # Error Handling
  ///
  /// - If the file cannot be opened or read, an error is logged and an empty stream is returned.
  /// - If the JSON cannot be deserialized, the error strategy determines the action.
  fn produce(&mut self) -> Self::OutputStream {
    let path = self.path.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "json_producer".to_string());
    let error_strategy = self.config.error_strategy.clone();
    let array_as_stream = self.json_config.array_as_stream;

    Box::pin(async_stream::stream! {
      // Read the entire file
      let content = match read_to_string(&path).await {
        Ok(content) => content,
        Err(e) => {
          let error = StreamError::new(
            Box::new(e),
            ErrorContext {
              timestamp: chrono::Utc::now(),
              item: None,
              component_name: component_name.clone(),
              component_type: std::any::type_name::<Self>().to_string(),
            },
            ComponentInfo {
              name: component_name.clone(),
              type_name: std::any::type_name::<Self>().to_string(),
            },
          );
          match handle_error_strategy(&error_strategy, &error) {
            ErrorAction::Stop => {
              error!(
                component = %component_name,
                path = %path,
                error = %error,
                "Failed to read file, stopping"
              );
            }
            ErrorAction::Skip => {
              warn!(
                component = %component_name,
                path = %path,
                error = %error,
                "Failed to read file, skipping"
              );
            }
            ErrorAction::Retry => {
              warn!(
                component = %component_name,
                path = %path,
                error = %error,
                "Retry not supported for file read errors, skipping"
              );
            }
          }
          return;
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
                    let error = StreamError::new(
                      Box::new(e),
                      ErrorContext {
                        timestamp: chrono::Utc::now(),
                        item: None,
                        component_name: component_name.clone(),
                        component_type: std::any::type_name::<Self>().to_string(),
                      },
                      ComponentInfo {
                        name: component_name.clone(),
                        type_name: std::any::type_name::<Self>().to_string(),
                      },
                    );
                    match handle_error_strategy(&error_strategy, &error) {
                      ErrorAction::Stop => {
                        error!(
                          component = %component_name,
                          index = idx,
                          error = %error,
                          "Stopping due to deserialization error at array index {}",
                          idx
                        );
                        return;
                      }
                      ErrorAction::Skip => {
                        warn!(
                          component = %component_name,
                          index = idx,
                          error = %error,
                          "Skipping array element at index {} due to deserialization error",
                          idx
                        );
                      }
                      ErrorAction::Retry => {
                        warn!(
                          component = %component_name,
                          index = idx,
                          error = %error,
                          "Retry not supported for array element deserialization, skipping"
                        );
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
                let error = StreamError::new(
                  Box::new(e),
                  ErrorContext {
                    timestamp: chrono::Utc::now(),
                    item: None,
                    component_name: component_name.clone(),
                    component_type: std::any::type_name::<Self>().to_string(),
                  },
                  ComponentInfo {
                    name: component_name.clone(),
                    type_name: std::any::type_name::<Self>().to_string(),
                  },
                );
                match handle_error_strategy(&error_strategy, &error) {
                  ErrorAction::Stop => {
                    error!(
                      component = %component_name,
                      error = %error,
                      "Stopping due to deserialization error"
                    );
                  }
                  ErrorAction::Skip => {
                    warn!(
                      component = %component_name,
                      error = %error,
                      "Skipping item due to deserialization error"
                    );
                  }
                  ErrorAction::Retry => {
                    warn!(
                      component = %component_name,
                      error = %error,
                      "Retry not supported for deserialization errors, skipping"
                    );
                  }
                }
              }
            }
          }
        }
        Err(e) => {
          let error = StreamError::new(
            Box::new(e),
            ErrorContext {
              timestamp: chrono::Utc::now(),
              item: None,
              component_name: component_name.clone(),
              component_type: std::any::type_name::<Self>().to_string(),
            },
            ComponentInfo {
              name: component_name.clone(),
              type_name: std::any::type_name::<Self>().to_string(),
            },
          );
          match handle_error_strategy(&error_strategy, &error) {
            ErrorAction::Stop => {
              error!(
                component = %component_name,
                error = %error,
                "Stopping due to JSON parse error"
              );
            }
            ErrorAction::Skip => {
              warn!(
                component = %component_name,
                error = %error,
                "Skipping due to JSON parse error"
              );
            }
            ErrorAction::Retry => {
              warn!(
                component = %component_name,
                error = %error,
                "Retry not supported for JSON parse errors, skipping"
              );
            }
          }
        }
      }
    })
  }

  fn set_config_impl(&mut self, config: ProducerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "json_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "json_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

/// Helper function to handle error strategy
fn handle_error_strategy<T>(strategy: &ErrorStrategy<T>, error: &StreamError<T>) -> ErrorAction
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
