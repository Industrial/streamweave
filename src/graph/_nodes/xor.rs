//! XOR node for performing exclusive OR (XOR) logic operations.
//!
//! This module provides [`Xor`], a graph node that performs exclusive OR (XOR)
//! logic operations on boolean values or bitwise XOR on numeric values. It wraps
//! [`XorTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`Xor`] is useful for performing XOR operations in graph-based pipelines.
//! It supports both boolean XOR (logical XOR) and bitwise XOR (numeric XOR),
//! making it ideal for logical operations, bit manipulation, and data processing.
//!
//! # Key Concepts
//!
//! - **XOR Operations**: Performs exclusive OR operations on values
//! - **Boolean Mode**: Logical XOR on boolean values
//! - **Bitwise Mode**: Bitwise XOR on numeric values
//! - **Transformer Wrapper**: Wraps `XorTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`Xor<T>`]**: Node that performs XOR operations
//! - **[`XorOps`]**: Enum representing different XOR operation types
//!
//! # Quick Start
//!
//! ## Basic Usage (Boolean XOR)
//!
//! ```rust
//! use streamweave::graph::nodes::Xor;
//!
//! // Create an XOR node for boolean operations
//! let xor = Xor::<bool>::new(true);
//! ```
//!
//! ## Bitwise XOR
//!
//! ```rust
//! use streamweave::graph::nodes::Xor;
//!
//! // Create an XOR node for bitwise operations
//! let xor = Xor::<u32>::new(0xFF);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::Xor;
//! use streamweave::ErrorStrategy;
//!
//! // Create an XOR node with error handling
//! let xor = Xor::<bool>::new(true)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("xor".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Type Flexibility**: Supports both boolean and numeric types for XOR
//! - **Operand-Based**: Uses an operand value for flexible XOR operations
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`Xor`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{XorOps, XorTransformer};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that performs XOR (exclusive OR) operations.
///
/// This node wraps `XorTransformer` for use in graphs. It can operate in two modes:
/// - Boolean mode: XOR on boolean values
/// - Bitwise mode: XOR on numeric values (bitwise XOR)
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{Xor, TransformerNode};
///
/// // Boolean XOR: true XOR false = true
/// let xor = Xor::new(true);
/// let node = TransformerNode::from_transformer(
///     "xor".to_string(),
///     xor,
/// );
/// ```
pub struct Xor<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying XOR transformer
  transformer: XorTransformer<T>,
}

impl<T> Xor<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Xor` node with the specified operand.
  ///
  /// Each input item will be XORed with this operand.
  ///
  /// # Arguments
  ///
  /// * `operand` - The value to XOR with each input item.
  pub fn new(operand: T) -> Self {
    Self {
      transformer: XorTransformer::new(operand),
    }
  }

  /// Sets the error handling strategy for this node.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }
}

impl<T> Clone for Xor<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for Xor<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for Xor<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for Xor<T>
where
  T: XorOps + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}
