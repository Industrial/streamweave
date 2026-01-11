//! # XOR Transformer
//!
//! Transformer that performs exclusive OR (XOR) operations on input values,
//! supporting both boolean logic XOR and bitwise XOR for numeric types. This
//! enables cryptographic operations, checksums, and logical transformations.
//!
//! ## Overview
//!
//! The XOR Transformer provides:
//!
//! - **Boolean XOR**: Logical XOR operation on boolean values
//! - **Bitwise XOR**: Bitwise XOR operation on numeric values
//! - **Configurable Operand**: XOR each input with a configurable operand value
//! - **Type Generic**: Works with any type that supports XOR operations
//! - **Error Handling**: Configurable error strategies
//!
//! ## Input/Output
//!
//! - **Input**: `Message<T>` - Values to XOR
//! - **Output**: `Message<T>` - XORed values
//!
//! ## Operation Modes
//!
//! - **Boolean Mode**: `true XOR false = true`, `true XOR true = false`
//! - **Bitwise Mode**: `5 XOR 3 = 6` (bitwise operation)
//!
//! ## Example
//!
//! ```rust
//! use streamweave::transformers::XorTransformer;
//!
//! // Boolean XOR: true XOR false = true
//! let transformer = XorTransformer::new(true);
//! // Input: [false, true, false]
//! // Output: [true, false, true]
//!
//! // Bitwise XOR: 5 XOR 3 = 6
//! let transformer = XorTransformer::new(3u32);
//! // Input: [5, 7, 2]
//! // Output: [6, 4, 1]
//! ```

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// A transformer that performs XOR (exclusive OR) operations.
///
/// This transformer can operate in two modes:
/// - Boolean mode: XOR on boolean values
/// - Bitwise mode: XOR on numeric values (bitwise XOR)
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::XorTransformer;
///
/// // Boolean XOR: true XOR false = true
/// let transformer = XorTransformer::new(true);
/// // Input: [false, true, false]
/// // Output: [true, false, true]
///
/// // Bitwise XOR: 5 XOR 3 = 6
/// let transformer = XorTransformer::new(3u32);
/// // Input: [5, 7, 2]
/// // Output: [6, 4, 1]
/// ```
pub struct XorTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The value to XOR with each input item
  operand: T,
  /// Configuration for the transformer
  config: TransformerConfig<T>,
}

impl<T> XorTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `XorTransformer` with the specified operand.
  ///
  /// Each input item will be XORed with this operand.
  ///
  /// # Arguments
  ///
  /// * `operand` - The value to XOR with each input item.
  pub fn new(operand: T) -> Self {
    Self {
      operand,
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<T> Clone for XorTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      operand: self.operand.clone(),
      config: self.config.clone(),
    }
  }
}

impl<T> Input for XorTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for XorTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

/// Helper trait for XOR operations
///
/// This trait enables XOR operations on types that support it.
/// Implemented for boolean and all integer types.
pub trait XorOps: Clone {
  fn xor(self, other: Self) -> Self;
}

impl XorOps for bool {
  fn xor(self, other: Self) -> Self {
    self ^ other
  }
}

macro_rules! impl_xor_ops {
  ($($t:ty),*) => {
    $(
      impl XorOps for $t {
        fn xor(self, other: Self) -> Self {
          self ^ other
        }
      }
    )*
  };
}

impl_xor_ops!(
  u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);

#[async_trait]
impl<T> Transformer for XorTransformer<T>
where
  T: XorOps + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let operand = self.operand.clone();
    Box::pin(input.map(move |item| item.xor(operand.clone())))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
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
        .unwrap_or_else(|| "xor_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
