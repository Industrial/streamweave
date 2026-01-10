//! JSON write transformer for StreamWeave
//!
//! Writes data to JSON files while passing data through. Takes data as input, writes to JSON,
//! and outputs the same data, enabling writing intermediate results while continuing processing.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde::Serialize;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::Mutex;
use tracing::{error, warn};

/// Configuration for JSON writing behavior in transformers.
#[derive(Debug, Clone)]
pub struct JsonWriteTransformerConfig {
  /// Whether to write items as a JSON array (true) or as separate JSON documents (false).
  /// When true, items are written incrementally as an array: `[item1, item2, ...]`
  /// When false, each item is written as a separate JSON value (not valid for a single file).
  pub as_array: bool,
  /// Whether to pretty-print JSON.
  pub pretty: bool,
  /// Buffer size for writing.
  pub buffer_size: usize,
}

impl Default for JsonWriteTransformerConfig {
  fn default() -> Self {
    Self {
      as_array: true,
      pretty: false,
      buffer_size: 8192,
    }
  }
}

impl JsonWriteTransformerConfig {
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

/// A transformer that writes data to JSON files while passing data through.
///
/// Each input item is written to the JSON file, and then the same item is output,
/// enabling writing intermediate results while continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::JsonWriteTransformer;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Record {
///     name: String,
///     age: u32,
/// }
///
/// let transformer = JsonWriteTransformer::<Record>::new("output.json");
/// // Input: [Record { name: "Alice", age: 30 }, ...]
/// // Writes to JSON and outputs: [Record { name: "Alice", age: 30 }, ...]
/// ```
pub struct JsonWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Path to the JSON file.
  path: PathBuf,
  /// JSON-specific configuration.
  json_config: JsonWriteTransformerConfig,
  /// Transformer configuration.
  config: TransformerConfig<T>,
  /// Writer handle (shared across stream items).
  writer: Arc<Mutex<Option<BufWriter<tokio::fs::File>>>>,
  /// Whether we've written the first item (for array formatting).
  first_item_written: Arc<Mutex<bool>>,
}

impl<T> JsonWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `JsonWriteTransformer` for the specified file path.
  ///
  /// # Arguments
  ///
  /// * `path` - Path to the JSON file to write.
  #[must_use]
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
      json_config: JsonWriteTransformerConfig::default(),
      config: TransformerConfig::default(),
      writer: Arc::new(Mutex::new(None)),
      first_item_written: Arc::new(Mutex::new(false)),
    }
  }

  /// Sets the error handling strategy for this transformer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
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

impl<T> Clone for JsonWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      path: self.path.clone(),
      json_config: self.json_config.clone(),
      config: self.config.clone(),
      writer: Arc::clone(&self.writer),
      first_item_written: Arc::clone(&self.first_item_written),
    }
  }
}

impl<T> Input for JsonWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for JsonWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for JsonWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let path = self.path.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "json_write_transformer".to_string());
    let error_strategy = self.config.error_strategy.clone();
    let json_config = self.json_config.clone();
    let writer = Arc::clone(&self.writer);
    let first_item_written = Arc::clone(&self.first_item_written);

    Box::pin(async_stream::stream! {
      // Open the file on first write
      {
        let mut writer_guard = writer.lock().await;
        if writer_guard.is_none() {
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

          let buf_writer = BufWriter::with_capacity(json_config.buffer_size, file);
          *writer_guard = Some(buf_writer);

          // Write array opening bracket if as_array is true
          if json_config.as_array
            && let Some(ref mut w) = *writer_guard
              && let Err(e) = w.write_all(b"[").await {
                error!(
                  component = %component_name,
                  error = %e,
                  "Failed to write array opening bracket"
                );
                return;
              }
        }
      }

      let mut input_stream = input;
      while let Some(item) = input_stream.next().await {
        // Write to JSON
        {
          let mut writer_guard = writer.lock().await;
          if let Some(ref mut w) = *writer_guard {
            let is_first = !*first_item_written.lock().await;

            // Write comma before item if not first and as_array is true
            if json_config.as_array && !is_first
              && let Err(e) = w.write_all(b",").await {
                error!(
                  component = %component_name,
                  error = %e,
                  "Failed to write comma separator"
                );
                continue;
              }

            // Serialize and write the item
            let json_result = if json_config.pretty {
              serde_json::to_string_pretty(&item)
            } else {
              serde_json::to_string(&item)
            };

            match json_result {
              Ok(json) => {
                if let Err(e) = w.write_all(json.as_bytes()).await {
                  let stream_error = StreamError::new(
                    Box::new(e),
                    ErrorContext {
                      timestamp: chrono::Utc::now(),
                      item: Some(item.clone()),
                      component_name: component_name.clone(),
                      component_type: std::any::type_name::<JsonWriteTransformer<T>>().to_string(),
                    },
                    ComponentInfo {
                      name: component_name.clone(),
                      type_name: std::any::type_name::<JsonWriteTransformer<T>>().to_string(),
                    },
                  );

                  match handle_error_strategy(&error_strategy, &stream_error) {
                    ErrorAction::Stop => {
                      error!(
                        component = %component_name,
                        error = %stream_error,
                        "Stopping due to JSON write error"
                      );
                      return;
                    }
                    ErrorAction::Skip => {
                      warn!(
                        component = %component_name,
                        error = %stream_error,
                        "Skipping item due to JSON write error"
                      );
                      continue;
                    }
                    ErrorAction::Retry => {
                      warn!(
                        component = %component_name,
                        error = %stream_error,
                        "Retry not supported for JSON write errors, skipping"
                      );
                      continue;
                    }
                  }
                }
              }
              Err(e) => {
                let stream_error = StreamError::new(
                  Box::new(e),
                  ErrorContext {
                    timestamp: chrono::Utc::now(),
                    item: Some(item.clone()),
                    component_name: component_name.clone(),
                    component_type: std::any::type_name::<JsonWriteTransformer<T>>().to_string(),
                  },
                  ComponentInfo {
                    name: component_name.clone(),
                    type_name: std::any::type_name::<JsonWriteTransformer<T>>().to_string(),
                  },
                );

                match handle_error_strategy(&error_strategy, &stream_error) {
                  ErrorAction::Stop => {
                    error!(
                      component = %component_name,
                      error = %stream_error,
                      "Stopping due to JSON serialization error"
                    );
                    return;
                  }
                  ErrorAction::Skip => {
                    warn!(
                      component = %component_name,
                      error = %stream_error,
                      "Skipping item due to JSON serialization error"
                    );
                    continue;
                  }
                  ErrorAction::Retry => {
                    warn!(
                      component = %component_name,
                      error = %stream_error,
                      "Retry not supported for JSON serialization errors, skipping"
                    );
                    continue;
                  }
                }
              }
            }

            *first_item_written.lock().await = true;
          }
        }

        // Pass through the item
        yield item;
      }

      // Write array closing bracket if as_array is true
      {
        let mut writer_guard = writer.lock().await;
        if let Some(ref mut w) = *writer_guard {
          if json_config.as_array
            && let Err(e) = w.write_all(b"]").await {
              error!(
                component = %component_name,
                error = %e,
                "Failed to write array closing bracket"
              );
            }

          // Final flush
          if let Err(e) = w.flush().await {
            error!(
              component = %component_name,
              error = %e,
              "Failed to flush JSON file"
            );
          }
        }
      }
    })
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
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
        .unwrap_or_else(|| "json_write_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "json_write_transformer".to_string()),
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
