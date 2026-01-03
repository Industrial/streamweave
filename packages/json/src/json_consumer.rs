use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde::Serialize;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::pin::Pin;
use streamweave::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave::{Consumer, ConsumerConfig, Input};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncWriteExt, BufWriter};
use tracing::{error, warn};

/// Configuration for JSON writing behavior.
#[derive(Debug, Clone)]
pub struct JsonWriteConfig {
  /// Whether to write items as a JSON array (true) or as separate JSON documents (false).
  /// When true, all items are written as a single JSON array: `[item1, item2, ...]`
  /// When false, each item is written as a separate JSON document (not valid for a single file).
  /// Note: Writing separate documents to a single file is not standard JSON, so this option
  /// is primarily for array mode.
  pub as_array: bool,
  /// Whether to pretty-print JSON.
  pub pretty: bool,
  /// Buffer size for writing.
  pub buffer_size: usize,
}

impl Default for JsonWriteConfig {
  fn default() -> Self {
    Self {
      as_array: true,
      pretty: false,
      buffer_size: 8192,
    }
  }
}

impl JsonWriteConfig {
  /// Sets whether to write items as a JSON array.
  #[must_use]
  pub fn with_as_array(mut self, as_array: bool) -> Self {
    self.as_array = as_array;
    self
  }

  /// Sets whether to pretty-print JSON.
  #[must_use]
  pub fn with_pretty(mut self, pretty: bool) -> Self {
    self.pretty = pretty;
    self
  }

  /// Sets the buffer size.
  #[must_use]
  pub fn with_buffer_size(mut self, size: usize) -> Self {
    self.buffer_size = size;
    self
  }
}

/// A consumer that writes JSON files.
///
/// Unlike JSONL (JSON Lines), this consumer writes complete JSON documents.
/// By default, it writes all items as a single JSON array: `[item1, item2, ...]`
///
/// # Example
///
/// ```ignore
/// use streamweave_json_consumer::JsonConsumer;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Event {
///     id: u32,
///     message: String,
/// }
///
/// // Write items as a JSON array
/// let consumer = JsonConsumer::<Event>::new("events.json");
///
/// // Or write with pretty printing
/// let consumer = JsonConsumer::<Event>::new("events.json")
///     .with_pretty(true);
/// ```
pub struct JsonConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Path to the JSON file.
  pub path: PathBuf,
  /// Consumer configuration.
  pub config: ConsumerConfig<T>,
  /// JSON-specific configuration.
  pub json_config: JsonWriteConfig,
  /// File handle (opened on first write).
  pub file: Option<BufWriter<tokio::fs::File>>,
  /// Phantom data for the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> JsonConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new JSON consumer for the specified file path.
  #[must_use]
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
      config: ConsumerConfig::default(),
      json_config: JsonWriteConfig::default(),
      file: None,
      _phantom: PhantomData,
    }
  }

  /// Sets the error strategy for the consumer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the consumer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }

  /// Sets whether to write items as a JSON array.
  #[must_use]
  pub fn with_as_array(mut self, as_array: bool) -> Self {
    self.json_config.as_array = as_array;
    self
  }

  /// Sets whether to pretty-print JSON.
  #[must_use]
  pub fn with_pretty(mut self, pretty: bool) -> Self {
    self.json_config.pretty = pretty;
    self
  }

  /// Sets the buffer size.
  #[must_use]
  pub fn with_buffer_size(mut self, size: usize) -> Self {
    self.json_config.buffer_size = size;
    self
  }

  /// Returns the file path.
  #[must_use]
  pub fn path(&self) -> &PathBuf {
    &self.path
  }
}

// Trait implementations for JsonConsumer

impl<T> Input for JsonConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

#[async_trait]
impl<T> Consumer for JsonConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);

  /// Consumes a stream and writes items as JSON to the file.
  ///
  /// # Behavior
  ///
  /// - If `as_array` is true (default), all items are collected and written as a single JSON array.
  /// - If `as_array` is false, only the first item is written as a single JSON value (subsequent items are ignored).
  ///
  /// # Error Handling
  ///
  /// - If the file cannot be opened, an error is logged and no data is written.
  /// - If an item cannot be serialized or written, the error strategy determines the action.
  async fn consume(&mut self, input: Self::InputStream) {
    let path = self.path.clone();
    let component_name = if self.config.name.is_empty() {
      "json_consumer".to_string()
    } else {
      self.config.name.clone()
    };
    let error_strategy = self.config.error_strategy.clone();
    let as_array = self.json_config.as_array;
    let pretty = self.json_config.pretty;
    let buffer_size = self.json_config.buffer_size;

    // Open the file
    let file_result = OpenOptions::new()
      .create(true)
      .write(true)
      .truncate(true)
      .open(&path)
      .await;

    let file = match file_result {
      Ok(f) => f,
      Err(e) => {
        error!(
          component = %component_name,
          path = %path.display(),
          error = %e,
          "Failed to open file for writing"
        );
        return;
      }
    };

    let mut writer = BufWriter::with_capacity(buffer_size, file);
    let mut input = std::pin::pin!(input);

    if as_array {
      // Collect all items into a vector
      let mut items = Vec::new();
      while let Some(item) = input.next().await {
        items.push(item);
      }

      // Serialize the entire array
      let json_result = if pretty {
        serde_json::to_string_pretty(&items)
      } else {
        serde_json::to_string(&items)
      };

      match json_result {
        Ok(json) => {
          if let Err(e) = writer.write_all(json.as_bytes()).await {
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
                  "Stopping due to write error"
                );
              }
              ErrorAction::Skip | ErrorAction::Retry => {
                warn!(
                  component = %component_name,
                  error = %error,
                  "Error writing JSON array, continuing"
                );
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
                "Stopping due to serialization error"
              );
            }
            ErrorAction::Skip | ErrorAction::Retry => {
              warn!(
                component = %component_name,
                error = %error,
                "Error serializing JSON array, continuing"
              );
            }
          }
        }
      }
    } else {
      // Write only the first item as a single JSON value
      if let Some(item) = input.next().await {
        let json_result = if pretty {
          serde_json::to_string_pretty(&item)
        } else {
          serde_json::to_string(&item)
        };

        match json_result {
          Ok(json) => {
            if let Err(e) = writer.write_all(json.as_bytes()).await {
              let error = StreamError::new(
                Box::new(e),
                ErrorContext {
                  timestamp: chrono::Utc::now(),
                  item: Some(item.clone()),
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
                    "Stopping due to write error"
                  );
                }
                ErrorAction::Skip | ErrorAction::Retry => {
                  warn!(
                    component = %component_name,
                    error = %error,
                    "Error writing JSON, continuing"
                  );
                }
              }
            }
          }
          Err(e) => {
            let error = StreamError::new(
              Box::new(e),
              ErrorContext {
                timestamp: chrono::Utc::now(),
                item: Some(item),
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
                  "Stopping due to serialization error"
                );
              }
              ErrorAction::Skip | ErrorAction::Retry => {
                warn!(
                  component = %component_name,
                  error = %error,
                  "Error serializing JSON, continuing"
                );
              }
            }
          }
        }
      }
    }

    // Flush the writer
    if let Err(e) = writer.flush().await {
      error!(
        component = %component_name,
        error = %e,
        "Failed to flush file"
      );
    }

    // Store the file handle
    self.file = Some(writer);
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy {
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
      component_name: self.config.name.clone(),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self.config.name.clone(),
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
