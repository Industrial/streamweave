//! # File System Directory List Transformer
//!
//! Transformer that lists directory entries from input directory paths, producing
//! a stream of file and subdirectory paths. Supports both recursive and non-recursive
//! directory traversal.
//!
//! ## Overview
//!
//! The File System Directory List Transformer provides:
//!
//! - **Directory Listing**: Lists files and subdirectories in input directories
//! - **Recursive Traversal**: Optional recursive directory walking
//! - **One-to-Many**: Each input directory can produce multiple output paths
//! - **Error Handling**: Configurable error strategies for listing failures
//!
//! ## Input/Output
//!
//! - **Input**: `Message<String>` - Directory paths to list
//! - **Output**: `Message<String>` - File and subdirectory paths found
//!
//! ## Configuration
//!
//! - **Recursive**: When true, recursively walks through subdirectories
//! - **Non-Recursive**: When false, only lists entries in the top-level directory
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::transformers::FsDirectoryListTransformer;
//!
//! // Non-recursive listing
//! let transformer = FsDirectoryListTransformer::new();
//! // Input: ["/path/to/dir1", "/path/to/dir2"]
//! // Output: ["/path/to/dir1/file1.txt", "/path/to/dir1/file2.txt", ...]
//! ```

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tokio::fs;
use tracing::error;

/// Configuration for directory listing behavior.
#[derive(Debug, Clone, Default)]
pub struct FsDirectoryListConfig {
  /// Whether to read directories recursively.
  pub recursive: bool,
}

impl FsDirectoryListConfig {
  /// Sets whether to read directories recursively.
  ///
  /// When true, recursively walks through subdirectories.
  /// When false, only lists entries in the top-level directory.
  #[must_use]
  pub fn with_recursive(mut self, recursive: bool) -> Self {
    self.recursive = recursive;
    self
  }
}

/// A transformer that lists directory entries from input paths.
///
/// Input: String (directory path)
/// Output: String (file/directory path)
///
/// This transformer takes directory paths as input and outputs paths to files and
/// subdirectories found in those directories. It supports both recursive and
/// non-recursive directory traversal.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::FsDirectoryListTransformer;
///
/// // Non-recursive listing
/// let transformer = FsDirectoryListTransformer::new();
/// // Input: ["/path/to/dir1", "/path/to/dir2"]
/// // Output: ["/path/to/dir1/file1.txt", "/path/to/dir1/file2.txt", "/path/to/dir2/file3.txt", ...]
///
/// // Recursive listing
/// let transformer = FsDirectoryListTransformer::new()
///     .with_recursive(true);
/// // Input: ["/path/to/dir1"]
/// // Output: [all files and subdirectories recursively]
/// ```
pub struct FsDirectoryListTransformer {
  /// Transformer configuration.
  pub config: TransformerConfig<String>,
  /// Directory listing configuration.
  pub dir_config: FsDirectoryListConfig,
}

impl FsDirectoryListTransformer {
  /// Creates a new `FsDirectoryListTransformer` with default configuration (non-recursive).
  #[must_use]
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
      dir_config: FsDirectoryListConfig::default(),
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

  /// Sets whether to read directories recursively.
  ///
  /// When true, recursively walks through subdirectories.
  /// When false, only lists entries in the top-level directory.
  #[must_use]
  pub fn with_recursive(mut self, recursive: bool) -> Self {
    self.dir_config = self.dir_config.with_recursive(recursive);
    self
  }
}

impl Default for FsDirectoryListTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for FsDirectoryListTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      dir_config: self.dir_config.clone(),
    }
  }
}

impl Input for FsDirectoryListTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for FsDirectoryListTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for FsDirectoryListTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let shared_self = std::sync::Arc::new(self.clone());
    let component_name = shared_self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "fs_directory_list_transformer".to_string());
    let recursive = shared_self.dir_config.recursive;
    let shared_self_clone = shared_self.clone();
    let component_name_clone = component_name.clone();

    Box::pin(input.flat_map(move |dir_path| {
      let cloned_self = shared_self_clone.clone();
      let component_name = component_name_clone.clone();

      async_stream::stream! {
        match fs::read_dir(&dir_path).await {
          Ok(mut entries) => {
            if recursive {
              // Recursive directory walk
              let mut stack = vec![dir_path.clone()];
              while let Some(current_dir) = stack.pop() {
                match fs::read_dir(&current_dir).await {
                  Ok(mut dir_entries) => {
                    while let Ok(Some(entry)) = dir_entries.next_entry().await {
                      if let Some(path_str) = entry.path().to_str() {
                        let path = path_str.to_string();
                        yield path.clone();
                        match entry.file_type().await {
                          Ok(file_type) if file_type.is_dir() => {
                            stack.push(path);
                          }
                          Ok(_) => {
                            // Not a directory, continue
                          }
                          Err(e) => {
                            let stream_error = StreamError::new(
                              Box::new(e),
                              ErrorContext {
                                timestamp: chrono::Utc::now(),
                                item: Some(path.clone()),
                                component_name: component_name.clone(),
                                component_type: std::any::type_name::<FsDirectoryListTransformer>().to_string(),
                              },
                              ComponentInfo {
                                name: component_name.clone(),
                                type_name: std::any::type_name::<FsDirectoryListTransformer>().to_string(),
                              },
                            );
                            match cloned_self.handle_error(&stream_error) {
                              ErrorAction::Stop => {
                                error!(
                                  component = %component_name,
                                  path = %path,
                                  error = %stream_error,
                                  "Stopping due to file type check error"
                                );
                                return;
                              }
                              ErrorAction::Skip => {
                                // Continue to next entry
                              }
                              ErrorAction::Retry => {
                                // Retry not directly supported for file type checks
                              }
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
                        item: Some(current_dir.clone()),
                        component_name: component_name.clone(),
                        component_type: std::any::type_name::<FsDirectoryListTransformer>().to_string(),
                      },
                      ComponentInfo {
                        name: component_name.clone(),
                        type_name: std::any::type_name::<FsDirectoryListTransformer>().to_string(),
                      },
                    );
                    match cloned_self.handle_error(&stream_error) {
                      ErrorAction::Stop => {
                        error!(
                          component = %component_name,
                          path = %current_dir,
                          error = %stream_error,
                          "Stopping due to directory read error"
                        );
                        return;
                      }
                      ErrorAction::Skip => {
                        // Skip directories that can't be read
                      }
                      ErrorAction::Retry => {
                        // Retry not directly supported for directory read errors
                      }
                    }
                  }
                }
              }
            } else {
              // Non-recursive: just yield entries in the top-level directory
              while let Ok(Some(entry)) = entries.next_entry().await {
                if let Some(path_str) = entry.path().to_str() {
                  yield path_str.to_string();
                }
              }
            }
          }
          Err(e) => {
            error!(
              component = %component_name,
              path = %dir_path,
              error = %e,
              "Failed to read directory"
            );
            let stream_error = StreamError::new(
              Box::new(e),
              ErrorContext {
                timestamp: chrono::Utc::now(),
                item: Some(dir_path.clone()),
                component_name: component_name.clone(),
                component_type: std::any::type_name::<FsDirectoryListTransformer>().to_string(),
              },
              ComponentInfo {
                name: component_name.clone(),
                type_name: std::any::type_name::<FsDirectoryListTransformer>().to_string(),
              },
            );
            match cloned_self.handle_error(&stream_error) {
              ErrorAction::Stop => {
                // Stop processing this directory
              }
              ErrorAction::Skip => {
                // Skip this directory and continue
              }
              ErrorAction::Retry => {
                // Retry not directly supported for directory read errors
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
        .unwrap_or_else(|| "fs_directory_list_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "fs_directory_list_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
