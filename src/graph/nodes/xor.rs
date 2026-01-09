//! XOR node for StreamWeave graphs
//!
//! Performs exclusive OR (XOR) logic operations on boolean values or bits.
//! This node can work with boolean values or numeric values (bitwise XOR).

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
