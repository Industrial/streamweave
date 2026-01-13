//! String trim node for StreamWeave graphs.
//!
//! This module provides [`StringTrim`], a graph node that removes whitespace from
//! strings based on a configurable trim mode. It wraps [`StringTrimTransformer`] for
//! use in StreamWeave graph topologies.
//!
//! # Overview
//!
//! [`StringTrim`] is useful for cleaning string data in graph-based processing pipelines.
//! It supports three trim modes: removing whitespace from both ends, only the leading
//! edge, or only the trailing edge of strings.
//!
//! # Key Concepts
//!
//! - **Whitespace Removal**: Removes Unicode whitespace characters from strings
//! - **Trim Modes**: Configurable trimming (Both, Left, Right)
//! - **Graph Integration**: Wraps transformer for use in graph topologies
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`StringTrim`]**: Graph node that trims whitespace from strings
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::StringTrim;
//! use streamweave::transformers::TrimMode;
//!
//! // Create a node that trims both leading and trailing whitespace
//! let node = StringTrim::new(TrimMode::Both)
//!     .with_name("trim-whitespace".to_string());
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::StringTrim;
//! use streamweave::transformers::TrimMode;
//! use streamweave::ErrorStrategy;
//!
//! // Create a node with error handling strategy
//! let node = StringTrim::new(TrimMode::Both)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("trim-node".to_string());
//! ```
//!
//! # Trim Modes
//!
//! - **Both**: Removes whitespace from both leading and trailing edges
//! - **Left**: Removes whitespace only from the leading edge
//! - **Right**: Removes whitespace only from the trailing edge
//!
//! # Design Decisions
//!
//! - **Transformer Wrapper**: Wraps `StringTrimTransformer` to provide graph node interface
//! - **Mode Configuration**: Trim mode specified at construction time
//! - **Unicode Support**: Uses Rust's standard library trim functions for Unicode whitespace
//! - **Graph Integration**: Seamlessly integrates with StreamWeave graph execution
//!
//! # Integration with StreamWeave
//!
//! [`StringTrim`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{StringTrimTransformer, TrimMode};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that trims whitespace from strings.
///
/// This node wraps `StringTrimTransformer` for use in graphs.
pub struct StringTrim {
  /// The underlying trim transformer
  transformer: StringTrimTransformer,
}

impl StringTrim {
  /// Creates a new `StringTrim` node with the specified mode.
  pub fn new(mode: TrimMode) -> Self {
    Self {
      transformer: StringTrimTransformer::new(mode),
    }
  }

  /// Sets the error handling strategy for this node.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }
}

impl Clone for StringTrim {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for StringTrim {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for StringTrim {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for StringTrim {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<String>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<String> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<String> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}
