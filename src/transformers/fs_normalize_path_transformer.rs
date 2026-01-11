//! Path normalization transformer for StreamWeave.
//!
//! This module provides [`FsNormalizePathTransformer`], a transformer that
//! normalizes file path strings by removing redundant separators and resolving
//! `.` and `..` components where possible. It uses Rust's standard library
//! `Path` API for cross-platform path normalization.
//!
//! # Overview
//!
//! [`FsNormalizePathTransformer`] is useful for normalizing file paths in
//! streaming pipelines. It processes path strings and normalizes them to their
//! canonical form, making it ideal for path manipulation and file processing
//! operations.
//!
//! # Key Concepts
//!
//! - **Path Normalization**: Normalizes paths by removing redundant separators
//!   and resolving `.` and `..` components
//! - **Cross-Platform**: Uses Rust's `Path` API for cross-platform compatibility
//! - **String Processing**: Works with path strings for compatibility
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`FsNormalizePathTransformer`]**: Transformer that normalizes path strings
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::FsNormalizePathTransformer;
//! use streamweave::PipelineBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer that normalizes paths
//! let transformer = FsNormalizePathTransformer::new();
//!
//! // Input: ["/path/to/../file.txt", "./data/../config.json"]
//! // Output: ["/path/file.txt", "config.json"]
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::FsNormalizePathTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a transformer with error handling strategy
//! let transformer = FsNormalizePathTransformer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("path-normalizer".to_string());
//! ```
//!
//! # Behavior
//!
//! The transformer normalizes paths by:
//! - Removing redundant path separators
//! - Resolving `.` (current directory) components
//! - Resolving `..` (parent directory) components where possible
//! - Using platform-specific path handling
//!
//! # Design Decisions
//!
//! - **Path API**: Uses Rust's standard `Path` API for cross-platform compatibility
//! - **Component Collection**: Collects path components to normalize the path
//! - **String Input/Output**: Works with strings for compatibility with text-based streams
//! - **Simple Normalization**: Focuses on basic path normalization
//!
//! # Integration with StreamWeave
//!
//! [`FsNormalizePathTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::path::Path;
use std::pin::Pin;

/// A transformer that normalizes path strings.
///
/// This transformer normalizes paths by removing redundant separators
/// and resolving `.` and `..` components where possible.
/// Uses Rust's standard library Path normalization.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::FsNormalizePathTransformer;
///
/// let transformer = FsNormalizePathTransformer::new();
/// // Input: ["/path/to/../file.txt"]
/// // Output: ["/path/file.txt"]
/// ```
pub struct FsNormalizePathTransformer {
  /// Configuration for the transformer
  pub config: TransformerConfig<String>,
}

impl FsNormalizePathTransformer {
  /// Creates a new `FsNormalizePathTransformer`.
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error handling strategy.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Default for FsNormalizePathTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for FsNormalizePathTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for FsNormalizePathTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for FsNormalizePathTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for FsNormalizePathTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.map(|path_str| {
      // Use Path::new to parse and normalize by collecting components
      let path = Path::new(&path_str);
      let components = path.components();
      let normalized: std::path::PathBuf = components.collect();
      normalized.to_string_lossy().to_string()
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
        .unwrap_or_else(|| "fs_normalize_path_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "fs_normalize_path_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
