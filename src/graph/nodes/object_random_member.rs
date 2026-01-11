//! Object random member node for getting a random member from an array.
//!
//! This module provides [`ObjectRandomMember`], a graph node that gets a random
//! member from an array. It wraps [`ObjectRandomMemberTransformer`] for use in
//! StreamWeave graphs. It selects a random element from arrays, making it ideal
//! for sampling and randomization.
//!
//! # Overview
//!
//! [`ObjectRandomMember`] is useful for getting a random member from arrays in
//! graph-based pipelines. It processes arrays and outputs a random element,
//! making it ideal for sampling, randomization, and stochastic processing.
//!
//! # Key Concepts
//!
//! - **Random Selection**: Selects a random member from arrays
//! - **Array Processing**: Works with JSON arrays
//! - **JSON Value Support**: Works with `serde_json::Value` arrays
//! - **Transformer Wrapper**: Wraps `ObjectRandomMemberTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`ObjectRandomMember`]**: Node that gets a random member from arrays
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::ObjectRandomMember;
//!
//! // Create an object random member node
//! let random_member = ObjectRandomMember::new();
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::ObjectRandomMember;
//! use streamweave::ErrorStrategy;
//!
//! // Create an object random member node with error handling
//! let random_member = ObjectRandomMember::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("random-selector".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Value Support**: Works with `serde_json::Value` for flexible JSON
//!   array handling
//! - **Random Selection**: Uses a random number generator for random selection
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`ObjectRandomMember`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::ObjectRandomMemberTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that gets a random member from an array.
///
/// This node wraps `ObjectRandomMemberTransformer` for use in graphs.
pub struct ObjectRandomMember {
  /// The underlying random member transformer
  transformer: ObjectRandomMemberTransformer,
}

impl ObjectRandomMember {
  /// Creates a new `ObjectRandomMember` node.
  pub fn new() -> Self {
    Self {
      transformer: ObjectRandomMemberTransformer::new(),
    }
  }

  /// Sets the error handling strategy for this node.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Value>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }
}

impl Default for ObjectRandomMember {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for ObjectRandomMember {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for ObjectRandomMember {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ObjectRandomMember {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ObjectRandomMember {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Value>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<Value> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Value> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<Value>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<Value>) -> ErrorContext<Value> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}
