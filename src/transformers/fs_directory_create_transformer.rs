//! # File System Directory Create Transformer
//!
//! Transformer that creates directories from input path strings while passing the same
//! paths through to the output stream. This enables creating directory structures
//! while continuing the main pipeline flow.
//!
//! ## Overview
//!
//! The File System Directory Create Transformer provides:
//!
//! - **Directory Creation**: Creates directories (with all parent directories) from paths
//! - **Pass-Through**: Outputs the same paths that were used to create directories
//! - **Recursive Creation**: Automatically creates parent directories as needed
//! - **Error Handling**: Configurable error strategies for creation failures
//!
//! ## Input/Output
//!
//! - **Input**: `Message<String>` - Directory paths to create
//! - **Output**: `Message<String>` - The same paths (pass-through)
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::transformers::FsDirectoryCreateTransformer;
//!
//! let transformer = FsDirectoryCreateTransformer::new();
//! // Input: ["/path/to/dir1", "/path/to/dir2", ...]
//! // Creates directories and outputs: ["/path/to/dir1", "/path/to/dir2", ...]
//! ```

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tokio::fs;
use tracing::{error, warn};

/// A transformer that creates directories from path strings while passing paths through.
///
/// Each input path string is used to create a directory (with all parent directories),
/// and then the same path is output, enabling creating directories and continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::FsDirectoryCreateTransformer;
///
/// let transformer = FsDirectoryCreateTransformer::new();
/// // Input: ["/path/to/dir1", "/path/to/dir2", ...]
/// // Creates directories and outputs: ["/path/to/dir1", "/path/to/dir2", ...]
/// ```
pub struct FsDirectoryCreateTransformer {
  /// Transformer configuration.
  config: TransformerConfig<String>,
}

impl FsDirectoryCreateTransformer {
  /// Creates a new `FsDirectoryCreateTransformer`.
  #[must_use]
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
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
}

impl Default for FsDirectoryCreateTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for FsDirectoryCreateTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for FsDirectoryCreateTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for FsDirectoryCreateTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for FsDirectoryCreateTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "fs_directory_create_transformer".to_string());
    let error_strategy = self.config.error_strategy.clone();

    Box::pin(async_stream::stream! {
      let mut input_stream = input;
      while let Some(path) = input_stream.next().await {
        // Create directory
        if let Err(e) = fs::create_dir_all(&path).await {
          let stream_error = StreamError::new(
            Box::new(e),
            ErrorContext {
              timestamp: chrono::Utc::now(),
              item: Some(path.clone()),
              component_name: component_name.clone(),
              component_type: std::any::type_name::<FsDirectoryCreateTransformer>().to_string(),
            },
            ComponentInfo {
              name: component_name.clone(),
              type_name: std::any::type_name::<FsDirectoryCreateTransformer>().to_string(),
            },
          );

          match handle_error_strategy(&error_strategy, &stream_error) {
            ErrorAction::Stop => {
              error!(
                component = %component_name,
                error = %stream_error,
                "Stopping due to directory creation error"
              );
              return;
            }
            ErrorAction::Skip => {
              warn!(
                component = %component_name,
                error = %stream_error,
                "Skipping path due to directory creation error"
              );
              continue;
            }
            ErrorAction::Retry => {
              warn!(
                component = %component_name,
                error = %stream_error,
                "Retry not supported for directory creation errors, skipping"
              );
              continue;
            }
          }
        }

        // Pass through the path
        yield path;
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
        .unwrap_or_else(|| "fs_directory_create_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "fs_directory_create_transformer".to_string()),
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
