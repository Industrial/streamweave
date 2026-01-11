//! # String Split Transformer
//!
//! This module provides [`StringSplitTransformer`], a transformer that splits input
//! strings by a specified delimiter, producing a stream where each substring becomes
//! a separate output item. Useful for parsing delimited data formats, CSV-like data,
//! and text processing operations.
//!
//! # Overview
//!
//! [`StringSplitTransformer`] is useful for splitting delimited strings into individual
//! substrings in streaming data processing pipelines. Each input string is split by the
//! specified delimiter, with each resulting substring becoming a separate output item.
//! This enables one-to-many transformations where a single input item can produce
//! multiple output items.
//!
//! # Key Concepts
//!
//! - **Delimiter Splitting**: Splits strings by a configurable delimiter string
//! - **One-to-Many Transformation**: Each input string can produce multiple output substrings
//! - **Flexible Delimiters**: Supports any string delimiter (not just single characters)
//! - **Stream Processing**: Processes strings in a streaming fashion
//! - **Error Handling**: Configurable error strategies for handling failures
//!
//! # Core Types
//!
//! - **[`StringSplitTransformer`]**: Transformer that splits strings by a delimiter
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::StringSplitTransformer;
//! use streamweave::PipelineBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer that splits by comma
//! let transformer = StringSplitTransformer::new(",");
//!
//! // Input: ["a,b,c", "x,y"]
//! // Output: ["a", "b", "c", "x", "y"]
//! # Ok(())
//! # }
//! ```
//!
//! ## Different Delimiters
//!
//! ```rust
//! use streamweave::transformers::StringSplitTransformer;
//!
//! // Split by semicolon
//! let transformer = StringSplitTransformer::new(";");
//! // Input: ["a;b;c"]
//! // Output: ["a", "b", "c"]
//!
//! // Split by multi-character delimiter
//! let transformer = StringSplitTransformer::new("::");
//! // Input: ["a::b::c"]
//! // Output: ["a", "b", "c"]
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::StringSplitTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a transformer with error handling strategy
//! let transformer = StringSplitTransformer::new(",")
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("csv-splitter".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **String Delimiter**: Uses string-based delimiter matching, not regex (for simplicity and performance)
//! - **One-to-Many**: Uses `flat_map` to enable one input string producing multiple output items
//! - **Simple API**: Delimiter specified at construction time for clarity
//! - **Stream Processing**: Processes items in a streaming fashion for efficiency
//! - **Performance**: Uses Rust's standard library string splitting methods
//!
//! # Integration with StreamWeave
//!
//! [`StringSplitTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline or graph. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::stream;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// A transformer that splits strings by a delimiter.
///
/// Takes each input string and splits it by the specified delimiter,
/// producing a stream of substrings.
///
/// # Example
///
/// ```rust
/// use crate::transformers::StringSplitTransformer;
///
/// let transformer = StringSplitTransformer::new(",");
/// // Input: ["a,b,c", "x,y"]
/// // Output: ["a", "b", "c", "x", "y"]
/// ```
pub struct StringSplitTransformer {
  /// Delimiter to split by
  delimiter: String,
  /// Configuration for the transformer
  config: TransformerConfig<String>,
}

impl StringSplitTransformer {
  /// Creates a new `StringSplitTransformer` with the specified delimiter.
  ///
  /// # Arguments
  ///
  /// * `delimiter` - The delimiter to split by.
  pub fn new(delimiter: impl Into<String>) -> Self {
    Self {
      delimiter: delimiter.into(),
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this transformer.
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

impl Clone for StringSplitTransformer {
  fn clone(&self) -> Self {
    Self {
      delimiter: self.delimiter.clone(),
      config: self.config.clone(),
    }
  }
}

impl Input for StringSplitTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringSplitTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for StringSplitTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let delimiter = self.delimiter.clone();
    Box::pin(input.flat_map(move |s| {
      let parts: Vec<String> = s.split(&delimiter).map(|part| part.to_string()).collect();
      stream::iter(parts)
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
      component_name: self.component_info().name,
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "string_split_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
