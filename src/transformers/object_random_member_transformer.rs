//! Object random member transformer for StreamWeave.
//!
//! This module provides [`ObjectRandomMemberTransformer`], a transformer that
//! extracts a random member from JSON arrays, producing a stream where each input
//! array yields one randomly selected element. This enables random sampling and
//! stochastic operations on array data.
//!
//! # Overview
//!
//! [`ObjectRandomMemberTransformer`] is useful for random sampling from arrays
//! in streaming pipelines. It processes JSON arrays and selects a random element
//! from each array, making it ideal for sampling, randomization, and stochastic
//! processing workflows.
//!
//! # Key Concepts
//!
//! - **Random Selection**: Selects a random element from input arrays
//! - **Uniform Distribution**: Uses uniform random distribution for fair selection
//! - **Array Processing**: Works with JSON arrays of any type
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`ObjectRandomMemberTransformer`]**: Transformer that selects random array members
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::ObjectRandomMemberTransformer;
//! use streamweave::PipelineBuilder;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer that selects random array members
//! let transformer = ObjectRandomMemberTransformer::new();
//!
//! // Input: [json!([1, 2, 3, 4, 5])]
//! // Output: [json!(3)] (random element, varies each run)
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::ObjectRandomMemberTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a transformer with error handling strategy
//! let transformer = ObjectRandomMemberTransformer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("random-sampler".to_string());
//! ```
//!
//! # Behavior
//!
//! The transformer selects a random element from each input array using uniform
//! distribution. Empty arrays produce `Value::Null`. Non-array values also
//! produce `Value::Null`.
//!
//! # Design Decisions
//!
//! - **Uniform Distribution**: Uses `rand::thread_rng()` for uniform random selection
//! - **Array-Only**: Works with JSON arrays, returns `Value::Null` for non-arrays
//! - **Empty Array Handling**: Returns `Value::Null` for empty arrays
//! - **Random Selection**: Uses random number generation for each selection
//! - **Simple Sampling**: Focuses on straightforward random selection
//!
//! # Integration with StreamWeave
//!
//! [`ObjectRandomMemberTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use rand::Rng;
use serde_json::Value;
use std::pin::Pin;

/// A transformer that gets a random member from an array.
///
/// Takes each input array and outputs a random element from it.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::ObjectRandomMemberTransformer;
///
/// let transformer = ObjectRandomMemberTransformer::new();
/// // Input: [[1, 2, 3, 4, 5]]
/// // Output: [3] (random element)
/// ```
pub struct ObjectRandomMemberTransformer {
  /// Configuration for the transformer
  config: TransformerConfig<Value>,
}

impl ObjectRandomMemberTransformer {
  /// Creates a new `ObjectRandomMemberTransformer`.
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this transformer.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Value>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Default for ObjectRandomMemberTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for ObjectRandomMemberTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for ObjectRandomMemberTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for ObjectRandomMemberTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for ObjectRandomMemberTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.map(|value| {
      if let Value::Array(arr) = value {
        if arr.is_empty() {
          Value::Null
        } else {
          let mut rng = rand::thread_rng();
          let index = rng.gen_range(0..arr.len());
          arr[index].clone()
        }
      } else {
        Value::Null
      }
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Value>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Value> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Value> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Value>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Value>) -> ErrorContext<Value> {
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
        .unwrap_or_else(|| "object_random_member_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
