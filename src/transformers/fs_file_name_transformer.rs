//! File name extraction transformer for StreamWeave.
//!
//! This module provides [`FsFileNameTransformer`], a transformer that extracts
//! the file name from path strings. It processes file paths and extracts just
//! the filename component, useful for path manipulation and file processing
//! operations.
//!
//! # Overview
//!
//! [`FsFileNameTransformer`] is useful for extracting filenames from full file
//! paths in streaming pipelines. It takes path strings as input and produces
//! just the filename component, making it ideal for path manipulation and
//! file-based processing workflows.
//!
//! # Key Concepts
//!
//! - **Path Extraction**: Extracts the filename component from full paths
//! - **Cross-Platform**: Uses Rust's `Path` API for cross-platform path handling
//! - **String Processing**: Works with path strings for compatibility
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`FsFileNameTransformer`]**: Transformer that extracts filenames from paths
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::FsFileNameTransformer;
//! use streamweave::PipelineBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer that extracts filenames
//! let transformer = FsFileNameTransformer::new();
//!
//! // Input: ["/path/to/file.txt", "/another/path/data.json"]
//! // Output: ["file.txt", "data.json"]
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::FsFileNameTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a transformer with error handling strategy
//! let transformer = FsFileNameTransformer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("filename-extractor".to_string());
//! ```
//!
//! # Behavior
//!
//! The transformer extracts the filename component from each input path string.
//! If a path doesn't have a filename component (e.g., a directory path ending
//! with a separator), it returns an empty string.
//!
//! # Design Decisions
//!
//! - **Path API**: Uses Rust's standard `Path` API for cross-platform compatibility
//! - **String Input/Output**: Works with strings for compatibility with text-based streams
//! - **Empty String Fallback**: Returns empty string for paths without filename component
//! - **Simple Extraction**: Focuses solely on filename extraction for clarity
//!
//! # Integration with StreamWeave
//!
//! [`FsFileNameTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::path::Path;
use std::pin::Pin;

/// A transformer that extracts the file name from path strings.
///
/// Takes file path strings as input and extracts just the filename component,
/// returning the filename without the directory path.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::FsFileNameTransformer;
///
/// let transformer = FsFileNameTransformer::new();
/// // Input: ["/path/to/file.txt"]
/// // Output: ["file.txt"]
/// ```
pub struct FsFileNameTransformer {
  pub config: TransformerConfig<String>,
}

impl FsFileNameTransformer {
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Default for FsFileNameTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for FsFileNameTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for FsFileNameTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for FsFileNameTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for FsFileNameTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.map(|path_str| {
      Path::new(&path_str)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_default()
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
    match self.config.error_strategy {
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
        .unwrap_or_else(|| "fs_file_name_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "fs_file_name_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
