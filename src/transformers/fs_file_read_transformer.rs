//! # File System File Read Transformer
//!
//! Transformer that reads file contents from input file paths, producing file data
//! as either lines (String) or bytes (Bytes) based on configuration.
//!
//! ## Overview
//!
//! The File System File Read Transformer provides:
//!
//! - **File Reading**: Reads file contents from file system paths
//! - **Reading Modes**: Supports line-by-line reading or raw byte reading
//! - **One-to-Many**: Each input file can produce multiple output items (lines or chunks)
//! - **Buffer Configuration**: Configurable buffer sizes for efficient reading
//! - **Error Handling**: Configurable error strategies for read failures
//!
//! ## Input/Output
//!
//! - **Input**: `Message<String>` - File paths to read
//! - **Output**: `Message<FileContent>` - File content (Line or Bytes)
//!
//! ## Reading Modes
//!
//! - **Line Mode**: Reads files line-by-line, producing `FileContent::Line(String)`
//! - **Byte Mode**: Reads files as raw bytes, producing `FileContent::Bytes(Bytes)`
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::transformers::FsFileReadTransformer;
//!
//! let transformer = FsFileReadTransformer::new();
//! // Input: ["file1.txt", "file2.txt"]
//! // Output: [FileContent::Line("line1"), FileContent::Line("line2"), ...]
//! ```

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tracing::error;

/// File content output type - either a line (String) or bytes (Bytes).
#[derive(Debug, Clone)]
pub enum FileContent {
  /// A line from the file (when reading as lines).
  Line(String),
  /// A chunk of bytes from the file (when reading as bytes).
  Bytes(Bytes),
}

impl FileContent {
  /// Converts the content to a String.
  ///
  /// For Line variant, returns the line directly.
  /// For Bytes variant, converts bytes to UTF-8 string (with lossy conversion).
  pub fn as_string(&self) -> String {
    match self {
      FileContent::Line(s) => s.clone(),
      FileContent::Bytes(b) => String::from_utf8_lossy(b).to_string(),
    }
  }

  /// Returns the content as bytes.
  ///
  /// For Line variant, converts the line to bytes.
  /// For Bytes variant, returns the bytes directly.
  pub fn as_bytes(&self) -> Bytes {
    match self {
      FileContent::Line(s) => Bytes::from(s.as_bytes().to_vec()),
      FileContent::Bytes(b) => b.clone(),
    }
  }
}

/// Configuration for file reading behavior.
#[derive(Debug, Clone)]
pub struct FsFileReadConfig {
  /// Whether to read as lines (true) or bytes (false).
  pub read_as_lines: bool,
  /// Buffer size for reading bytes (only used when read_as_lines is false).
  pub buffer_size: usize,
}

impl Default for FsFileReadConfig {
  fn default() -> Self {
    Self {
      read_as_lines: true,
      buffer_size: 8192,
    }
  }
}

impl FsFileReadConfig {
  /// Sets whether to read as lines.
  ///
  /// When true, reads the file line by line, emitting each line as FileContent::Line.
  /// When false, reads the file as raw bytes, emitting chunks as FileContent::Bytes.
  #[must_use]
  pub fn with_read_as_lines(mut self, read_as_lines: bool) -> Self {
    self.read_as_lines = read_as_lines;
    self
  }

  /// Sets the buffer size for reading bytes.
  ///
  /// Only used when read_as_lines is false.
  #[must_use]
  pub fn with_buffer_size(mut self, buffer_size: usize) -> Self {
    self.buffer_size = buffer_size;
    self
  }
}

/// A transformer that reads file contents from input paths.
///
/// Input: String (file path)
/// Output: FileContent (Line or Bytes)
///
/// This transformer can read files in two modes:
/// - Lines mode: Reads the file line by line, emitting each line as FileContent::Line
/// - Bytes mode: Reads the file as raw bytes, emitting chunks as FileContent::Bytes
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::{FsFileReadTransformer, FileContent};
///
/// // Read as lines
/// let transformer = FsFileReadTransformer::new();
/// // Input: ["file1.txt", "file2.txt"]
/// // Output: [FileContent::Line("line1"), FileContent::Line("line2"), ...]
///
/// // Read as bytes
/// let transformer = FsFileReadTransformer::new()
///     .with_read_as_lines(false);
/// // Input: ["file1.bin", "file2.bin"]
/// // Output: [FileContent::Bytes(...), FileContent::Bytes(...), ...]
/// ```
pub struct FsFileReadTransformer {
  /// Transformer configuration.
  pub config: TransformerConfig<String>,
  /// File reading configuration.
  pub file_config: FsFileReadConfig,
}

impl FsFileReadTransformer {
  /// Creates a new `FsFileReadTransformer` with default configuration (reads as lines).
  #[must_use]
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
      file_config: FsFileReadConfig::default(),
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

  /// Sets whether to read as lines.
  ///
  /// When true, reads the file line by line, emitting each line as FileContent::Line.
  /// When false, reads the file as raw bytes, emitting chunks as FileContent::Bytes.
  #[must_use]
  pub fn with_read_as_lines(mut self, read_as_lines: bool) -> Self {
    self.file_config = self.file_config.with_read_as_lines(read_as_lines);
    self
  }

  /// Sets the buffer size for reading bytes.
  ///
  /// Only used when read_as_lines is false.
  #[must_use]
  pub fn with_buffer_size(mut self, buffer_size: usize) -> Self {
    self.file_config = self.file_config.with_buffer_size(buffer_size);
    self
  }
}

impl Default for FsFileReadTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for FsFileReadTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      file_config: self.file_config.clone(),
    }
  }
}

impl Input for FsFileReadTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for FsFileReadTransformer {
  type Output = FileContent;
  type OutputStream = Pin<Box<dyn Stream<Item = FileContent> + Send>>;
}

#[async_trait]
impl Transformer for FsFileReadTransformer {
  type InputPorts = (String,);
  type OutputPorts = (FileContent,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "fs_file_read_transformer".to_string());
    let error_strategy = self.config.error_strategy.clone();
    let read_as_lines = self.file_config.read_as_lines;
    let buffer_size = self.file_config.buffer_size;

    Box::pin(input.flat_map(move |path| {
      let component_name_clone = component_name.clone();
      let error_strategy_clone = error_strategy.clone();

      async_stream::stream! {
        match File::open(&path).await {
          Ok(file) => {
            if read_as_lines {
              // Read as lines
              let reader = BufReader::new(file);
              let mut lines = reader.lines();

              loop {
                match lines.next_line().await {
                  Ok(Some(line)) => yield FileContent::Line(line),
                  Ok(None) => break,
                  Err(e) => {
                    let stream_error = StreamError::new(
                      Box::new(e),
                      ErrorContext {
                        timestamp: chrono::Utc::now(),
                        item: Some(path.clone()),
                        component_name: component_name_clone.clone(),
                        component_type: std::any::type_name::<FsFileReadTransformer>().to_string(),
                      },
                      ComponentInfo {
                        name: component_name_clone.clone(),
                        type_name: std::any::type_name::<FsFileReadTransformer>().to_string(),
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
            } else {
              // Read as bytes
              let mut reader = BufReader::new(file);
              let mut buffer = vec![0u8; buffer_size];

              loop {
                match reader.read(&mut buffer).await {
                  Ok(0) => break, // EOF
                  Ok(n) => {
                    yield FileContent::Bytes(Bytes::from(buffer[..n].to_vec()));
                  }
                  Err(e) => {
                    let stream_error = StreamError::new(
                      Box::new(e),
                      ErrorContext {
                        timestamp: chrono::Utc::now(),
                        item: Some(path.clone()),
                        component_name: component_name_clone.clone(),
                        component_type: std::any::type_name::<FsFileReadTransformer>().to_string(),
                      },
                      ComponentInfo {
                        name: component_name_clone.clone(),
                        type_name: std::any::type_name::<FsFileReadTransformer>().to_string(),
                      },
                    );
                    match handle_error_strategy(&error_strategy_clone, &stream_error) {
                      ErrorAction::Stop => {
                        error!(
                          component = %component_name_clone,
                          path = %path,
                          error = %stream_error,
                          "Stopping due to byte read error"
                        );
                        break;
                      }
                      ErrorAction::Skip => {
                        // Continue to next chunk
                      }
                      ErrorAction::Retry => {
                        // Retry not directly supported for byte read errors
                      }
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
              "Failed to open file"
            );
            let stream_error = StreamError::new(
              Box::new(e),
              ErrorContext {
                timestamp: chrono::Utc::now(),
                item: Some(path.clone()),
                component_name: component_name_clone.clone(),
                component_type: std::any::type_name::<FsFileReadTransformer>().to_string(),
              },
              ComponentInfo {
                name: component_name_clone.clone(),
                type_name: std::any::type_name::<FsFileReadTransformer>().to_string(),
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
        .unwrap_or_else(|| "fs_file_read_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "fs_file_read_transformer".to_string()),
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
