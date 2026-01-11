//! Math random node for generating random numbers.
//!
//! This module provides [`MathRandomNode`], a graph node that generates random numbers.
//! It wraps [`MathRandomTransformer`] for use in StreamWeave graphs. It generates random
//! numbers within a specified range for each input item.
//!
//! # Overview
//!
//! [`MathRandomNode`] is useful for generating random numbers in graph-based pipelines.
//! It processes input items and generates random numbers within a specified range,
//! making it ideal for testing, sampling, or adding randomness to data processing.
//!
//! # Key Concepts
//!
//! - **Random Number Generation**: Generates random numbers within a specified range
//! - **Range-Based**: Supports min and max values for random number generation
//! - **Per-Item Generation**: Generates a random number for each input item
//! - **Transformer Wrapper**: Wraps `MathRandomTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`MathRandomNode`]**: Node that generates random numbers
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::MathRandomNode;
//!
//! // Generate random numbers between 0 and 100
//! let random = MathRandomNode::new(0.0, 100.0);
//!
//! // Generate random numbers between -1 and 1
//! let normalized = MathRandomNode::new(-1.0, 1.0);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::MathRandomNode;
//! use streamweave::ErrorStrategy;
//!
//! // Create a random node with error handling
//! let random = MathRandomNode::new(0.0, 100.0)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("random-generator".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **JSON Value Support**: Works with `serde_json::Value` for flexible numeric
//!   value handling
//! - **Range-Based**: Uses min (inclusive) and max (exclusive) for range specification
//! - **Random Generation**: Uses a random number generator for random value generation
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`MathRandomNode`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::MathRandomTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;
use std::pin::Pin;

/// Node that generates random numbers.
///
/// This node wraps `MathRandomTransformer` for use in graphs.
pub struct MathRandomNode {
  /// The underlying random transformer
  transformer: MathRandomTransformer,
}

impl MathRandomNode {
  /// Creates a new `MathRandomNode`.
  ///
  /// # Arguments
  ///
  /// * `min` - Minimum value (inclusive).
  /// * `max` - Maximum value (exclusive).
  pub fn new(min: f64, max: f64) -> Self {
    Self {
      transformer: MathRandomTransformer::new(min, max),
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

impl Clone for MathRandomNode {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for MathRandomNode {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for MathRandomNode {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for MathRandomNode {
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
