//! JSONL write transformer for StreamWeave
//!
//! Writes data to JSON Lines (JSONL) files while passing data through. Takes data as input, writes to JSONL,
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

/// Configuration for JSONL writing behavior in transformers.
#[derive(Debug, Clone)]
pub struct JsonlWriteTransformerConfig {
  /// Whether to append to existing file or overwrite.
  pub append: bool,
  /// Buffer size for writing.
  pub buffer_size: usize,
}

impl Default for JsonlWriteTransformerConfig {
  fn default() -> Self {
    Self {
      append: false,
      buffer_size: 8192,
    }
  }
}

impl JsonlWriteTransformerConfig {
  /// Sets whether to append to existing file.
  #[must_use]
  pub fn with_append(mut self, append: bool) -> Self {
    self.append = append;
    self
  }

  /// Sets the buffer size.
  #[must_use]
  pub fn with_buffer_size(mut self, size: usize) -> Self {
    self.buffer_size = size;
    self
  }
}

/// A transformer that writes data to JSON Lines (JSONL) files while passing data through.
///
/// Each input item is written as a JSON line (JSON value + newline) to the JSONL file,
/// and then the same item is output, enabling writing intermediate results while continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::JsonlWriteTransformer;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Record {
///     name: String,
///     age: u32,
/// }
///
/// let transformer = JsonlWriteTransformer::<Record>::new("output.jsonl");
/// // Input: [Record { name: "Alice", age: 30 }, ...]
/// // Writes to JSONL and outputs: [Record { name: "Alice", age: 30 }, ...]
/// ```
pub struct JsonlWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Path to the JSONL file.
  path: PathBuf,
  /// JSONL-specific configuration.
  jsonl_config: JsonlWriteTransformerConfig,
  /// Transformer configuration.
  config: TransformerConfig<T>,
  /// Writer handle (shared across stream items).
  writer: Arc<Mutex<Option<BufWriter<tokio::fs::File>>>>,
}

impl<T> JsonlWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `JsonlWriteTransformer` for the specified file path.
  ///
  /// # Arguments
  ///
  /// * `path` - Path to the JSONL file to write.
  #[must_use]
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
      jsonl_config: JsonlWriteTransformerConfig::default(),
      config: TransformerConfig::default(),
      writer: Arc::new(Mutex::new(None)),
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

  /// Sets whether to append to existing file.
  #[must_use]
  pub fn with_append(mut self, append: bool) -> Self {
    self.jsonl_config.append = append;
    self
  }

  /// Sets the buffer size.
  #[must_use]
  pub fn with_buffer_size(mut self, size: usize) -> Self {
    self.jsonl_config.buffer_size = size;
    self
  }

  /// Returns the file path.
  #[must_use]
  pub fn path(&self) -> &PathBuf {
    &self.path
  }
}

impl<T> Clone for JsonlWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      path: self.path.clone(),
      jsonl_config: self.jsonl_config.clone(),
      config: self.config.clone(),
      writer: Arc::clone(&self.writer),
    }
  }
}

impl<T> Input for JsonlWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for JsonlWriteTransformer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for JsonlWriteTransformer<T>
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
      .unwrap_or_else(|| "jsonl_write_transformer".to_string());
    let error_strategy = self.config.error_strategy.clone();
    let jsonl_config = self.jsonl_config.clone();
    let writer = Arc::clone(&self.writer);

    Box::pin(async_stream::stream! {
      // Open the file on first write
      {
        let mut writer_guard = writer.lock().await;
        if writer_guard.is_none() {
          let file_result = OpenOptions::new()
            .create(true)
            .write(true)
            .append(jsonl_config.append)
            .truncate(!jsonl_config.append)
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

          let buf_writer = BufWriter::with_capacity(jsonl_config.buffer_size, file);
          *writer_guard = Some(buf_writer);
        }
      }

      let mut input_stream = input;
      while let Some(item) = input_stream.next().await {
        // Write to JSONL
        {
          let mut writer_guard = writer.lock().await;
          if let Some(ref mut w) = *writer_guard {
            // Serialize the item to JSON
            let json_result = serde_json::to_string(&item);

            match json_result {
              Ok(json) => {
                // Write the JSON line
                if let Err(e) = w.write_all(json.as_bytes()).await {
                  let stream_error = StreamError::new(
                    Box::new(e),
                    ErrorContext {
                      timestamp: chrono::Utc::now(),
                      item: Some(item.clone()),
                      component_name: component_name.clone(),
                      component_type: std::any::type_name::<JsonlWriteTransformer<T>>().to_string(),
                    },
                    ComponentInfo {
                      name: component_name.clone(),
                      type_name: std::any::type_name::<JsonlWriteTransformer<T>>().to_string(),
                    },
                  );

                  match handle_error_strategy(&error_strategy, &stream_error) {
                    ErrorAction::Stop => {
                      error!(
                        component = %component_name,
                        error = %stream_error,
                        "Stopping due to JSONL write error"
                      );
                      return;
                    }
                    ErrorAction::Skip => {
                      warn!(
                        component = %component_name,
                        error = %stream_error,
                        "Skipping item due to JSONL write error"
                      );
                      continue;
                    }
                    ErrorAction::Retry => {
                      warn!(
                        component = %component_name,
                        error = %stream_error,
                        "Retry not supported for JSONL write errors, skipping"
                      );
                      continue;
                    }
                  }
                }

                // Write newline
                if let Err(e) = w.write_all(b"\n").await {
                  let stream_error = StreamError::new(
                    Box::new(e),
                    ErrorContext {
                      timestamp: chrono::Utc::now(),
                      item: Some(item.clone()),
                      component_name: component_name.clone(),
                      component_type: std::any::type_name::<JsonlWriteTransformer<T>>().to_string(),
                    },
                    ComponentInfo {
                      name: component_name.clone(),
                      type_name: std::any::type_name::<JsonlWriteTransformer<T>>().to_string(),
                    },
                  );

                  match handle_error_strategy(&error_strategy, &stream_error) {
                    ErrorAction::Stop => {
                      error!(
                        component = %component_name,
                        error = %stream_error,
                        "Stopping due to newline write error"
                      );
                      return;
                    }
                    ErrorAction::Skip => {
                      warn!(
                        component = %component_name,
                        error = %stream_error,
                        "Skipping item due to newline write error"
                      );
                      continue;
                    }
                    ErrorAction::Retry => {
                      warn!(
                        component = %component_name,
                        error = %stream_error,
                        "Retry not supported for newline write errors, skipping"
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
                    component_type: std::any::type_name::<JsonlWriteTransformer<T>>().to_string(),
                  },
                  ComponentInfo {
                    name: component_name.clone(),
                    type_name: std::any::type_name::<JsonlWriteTransformer<T>>().to_string(),
                  },
                );

                match handle_error_strategy(&error_strategy, &stream_error) {
                  ErrorAction::Stop => {
                    error!(
                      component = %component_name,
                      error = %stream_error,
                      "Stopping due to JSONL serialization error"
                    );
                    return;
                  }
                  ErrorAction::Skip => {
                    warn!(
                      component = %component_name,
                      error = %stream_error,
                      "Skipping item due to JSONL serialization error"
                    );
                    continue;
                  }
                  ErrorAction::Retry => {
                    warn!(
                      component = %component_name,
                      error = %stream_error,
                      "Retry not supported for JSONL serialization errors, skipping"
                    );
                    continue;
                  }
                }
              }
            }
          }
        }

        // Pass through the item
        yield item;
      }

      // Final flush
      {
        let mut writer_guard = writer.lock().await;
        if let Some(ref mut w) = *writer_guard
          && let Err(e) = w.flush().await {
            error!(
              component = %component_name,
              error = %e,
              "Failed to flush JSONL file"
            );
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
        .unwrap_or_else(|| "jsonl_write_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "jsonl_write_transformer".to_string()),
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
