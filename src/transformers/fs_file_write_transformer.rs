//! File write transformer for StreamWeave
//!
//! Writes data to files while passing data through. Takes string data as input, writes to file,
//! and outputs the same data, enabling writing intermediate results while continuing processing.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::Mutex;
use tracing::{error, warn};

/// Configuration for file writing behavior in transformers.
#[derive(Debug, Clone)]
pub struct FsFileWriteTransformerConfig {
  /// Whether to append to existing file or overwrite.
  pub append: bool,
  /// Buffer size for writing.
  pub buffer_size: usize,
}

impl Default for FsFileWriteTransformerConfig {
  fn default() -> Self {
    Self {
      append: false,
      buffer_size: 8192,
    }
  }
}

impl FsFileWriteTransformerConfig {
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

/// A transformer that writes string data to files while passing data through.
///
/// Each input string is written to the file, and then the same string is output,
/// enabling writing intermediate results while continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::FsFileWriteTransformer;
///
/// let transformer = FsFileWriteTransformer::new("output.txt");
/// // Input: ["line1", "line2", ...]
/// // Writes to file and outputs: ["line1", "line2", ...]
/// ```
pub struct FsFileWriteTransformer {
  /// Path to the file.
  path: PathBuf,
  /// File-specific configuration.
  file_config: FsFileWriteTransformerConfig,
  /// Transformer configuration.
  config: TransformerConfig<String>,
  /// Writer handle (shared across stream items).
  writer: Arc<Mutex<Option<BufWriter<tokio::fs::File>>>>,
}

impl FsFileWriteTransformer {
  /// Creates a new `FsFileWriteTransformer` for the specified file path.
  ///
  /// # Arguments
  ///
  /// * `path` - Path to the file to write.
  #[must_use]
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
      file_config: FsFileWriteTransformerConfig::default(),
      config: TransformerConfig::default(),
      writer: Arc::new(Mutex::new(None)),
    }
  }

  /// Sets the error handling strategy for this transformer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
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
    self.file_config.append = append;
    self
  }

  /// Sets the buffer size.
  #[must_use]
  pub fn with_buffer_size(mut self, size: usize) -> Self {
    self.file_config.buffer_size = size;
    self
  }

  /// Returns the file path.
  #[must_use]
  pub fn path(&self) -> &PathBuf {
    &self.path
  }
}

impl Clone for FsFileWriteTransformer {
  fn clone(&self) -> Self {
    Self {
      path: self.path.clone(),
      file_config: self.file_config.clone(),
      config: self.config.clone(),
      writer: Arc::clone(&self.writer),
    }
  }
}

impl Input for FsFileWriteTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for FsFileWriteTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for FsFileWriteTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let path = self.path.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "fs_file_write_transformer".to_string());
    let error_strategy = self.config.error_strategy.clone();
    let file_config = self.file_config.clone();
    let writer = Arc::clone(&self.writer);

    Box::pin(async_stream::stream! {
      // Open the file on first write
      {
        let mut writer_guard = writer.lock().await;
        if writer_guard.is_none() {
          let file_result = OpenOptions::new()
            .create(true)
            .write(true)
            .append(file_config.append)
            .truncate(!file_config.append)
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

          let buf_writer = BufWriter::with_capacity(file_config.buffer_size, file);
          *writer_guard = Some(buf_writer);
        }
      }

      let mut input_stream = input;
      while let Some(item) = input_stream.next().await {
        // Write to file
        {
          let mut writer_guard = writer.lock().await;
          if let Some(ref mut w) = *writer_guard {
            if let Err(e) = w.write_all(item.as_bytes()).await {
              let stream_error = StreamError::new(
                Box::new(e),
                ErrorContext {
                  timestamp: chrono::Utc::now(),
                  item: Some(item.clone()),
                  component_name: component_name.clone(),
                  component_type: std::any::type_name::<FsFileWriteTransformer>().to_string(),
                },
                ComponentInfo {
                  name: component_name.clone(),
                  type_name: std::any::type_name::<FsFileWriteTransformer>().to_string(),
                },
              );

              match handle_error_strategy(&error_strategy, &stream_error) {
                ErrorAction::Stop => {
                  error!(
                    component = %component_name,
                    error = %stream_error,
                    "Stopping due to file write error"
                  );
                  return;
                }
                ErrorAction::Skip => {
                  warn!(
                    component = %component_name,
                    error = %stream_error,
                    "Skipping item due to file write error"
                  );
                  continue;
                }
                ErrorAction::Retry => {
                  warn!(
                    component = %component_name,
                    error = %stream_error,
                    "Retry not supported for file write errors, skipping"
                  );
                  continue;
                }
              }
            }

            // Flush after each write
            if let Err(e) = w.flush().await {
              let stream_error = StreamError::new(
                Box::new(e),
                ErrorContext {
                  timestamp: chrono::Utc::now(),
                  item: Some(item.clone()),
                  component_name: component_name.clone(),
                  component_type: std::any::type_name::<FsFileWriteTransformer>().to_string(),
                },
                ComponentInfo {
                  name: component_name.clone(),
                  type_name: std::any::type_name::<FsFileWriteTransformer>().to_string(),
                },
              );

              match handle_error_strategy(&error_strategy, &stream_error) {
                ErrorAction::Stop => {
                  error!(
                    component = %component_name,
                    error = %stream_error,
                    "Stopping due to file flush error"
                  );
                  return;
                }
                ErrorAction::Skip => {
                  warn!(
                    component = %component_name,
                    error = %stream_error,
                    "Skipping item due to file flush error"
                  );
                  continue;
                }
                ErrorAction::Retry => {
                  warn!(
                    component = %component_name,
                    error = %stream_error,
                    "Retry not supported for file flush errors, skipping"
                  );
                  continue;
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
              "Failed to perform final flush"
            );
          }
      }
    })
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
        .unwrap_or_else(|| "fs_file_write_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "fs_file_write_transformer".to_string()),
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
