//! Map transformer for StreamWeave.
//!
//! This module provides [`MapTransformer`], a transformer that applies a function
//! to each item in a stream, creating a one-to-one mapping between input and output
//! items. It's one of the most fundamental and commonly used transformers in
//! StreamWeave pipelines.
//!
//! # Overview
//!
//! [`MapTransformer`] is the core transformation primitive in StreamWeave. It takes
//! each input item, applies a user-defined function to it, and produces a transformed
//! output item. This enables arbitrary data transformations while maintaining the
//! stream's structure.
//!
//! # Key Concepts
//!
//! - **One-to-One Mapping**: Each input item produces exactly one output item
//! - **Function Application**: Applies a user-defined function to each item
//! - **Type Transformation**: Can transform items from one type to another
//! - **Zero-Copy Support**: Implements `ZeroCopyTransformer` for performance
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`MapTransformer<F, I, O>`]**: Transformer that applies a function to each item
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::MapTransformer;
//! use streamweave::PipelineBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer that doubles each number
//! let transformer = MapTransformer::new(|x: i32| x * 2);
//!
//! // Input: [1, 2, 3]
//! // Output: [2, 4, 6]
//! # Ok(())
//! # }
//! ```
//!
//! ## Type Transformation
//!
//! ```rust
//! use streamweave::transformers::MapTransformer;
//!
//! // Transform strings to their lengths
//! let transformer = MapTransformer::new(|s: String| s.len());
//!
//! // Input: ["hello", "world"]
//! // Output: [5, 5]
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::MapTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a transformer with error handling strategy
//! let transformer = MapTransformer::new(|x: i32| x * 2)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("double-values".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Generic Function Type**: Accepts any function that implements `FnMut(I) -> O`
//! - **Clone Support**: Function must be `Clone` for use in async contexts
//! - **Zero-Copy Support**: Implements `ZeroCopyTransformer` for performance optimization
//! - **Type Safety**: Strongly typed input and output types
//! - **Flexibility**: Can transform between any compatible types
//!
//! # Integration with StreamWeave
//!
//! [`MapTransformer`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave pipeline. It also implements [`ZeroCopyTransformer`] for performance
//! optimization in zero-copy execution modes. It supports the standard error handling
//! strategies and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::graph::ZeroCopyTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::borrow::Cow;
use std::pin::Pin;

/// A transformer that applies a function to each item in the stream.
///
/// This transformer takes each input item and applies a function to transform it
/// into an output item, creating a one-to-one mapping.
pub struct MapTransformer<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The function to apply to each input item.
  pub f: F,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<I>,
  /// Phantom data to track the input type parameter.
  pub _phantom_i: std::marker::PhantomData<I>,
  /// Phantom data to track the output type parameter.
  pub _phantom_o: std::marker::PhantomData<O>,
}

impl<F, I, O> MapTransformer<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `MapTransformer` with the given function.
  ///
  /// # Arguments
  ///
  /// * `f` - The function to apply to each input item.
  pub fn new(f: F) -> Self {
    Self {
      f,
      config: TransformerConfig::default(),
      _phantom_i: std::marker::PhantomData,
      _phantom_o: std::marker::PhantomData,
    }
  }

  /// Sets the error handling strategy for this transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<I>) -> Self {
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

impl<F, I, O> Clone for MapTransformer<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      f: self.f.clone(),
      config: self.config.clone(),
      _phantom_i: std::marker::PhantomData,
      _phantom_o: std::marker::PhantomData,
    }
  }
}

impl<F, I, O> Input for MapTransformer<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = I;
  type InputStream = Pin<Box<dyn Stream<Item = I> + Send>>;
}

impl<F, I, O> Output for MapTransformer<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = O;
  type OutputStream = Pin<Box<dyn Stream<Item = O> + Send>>;
}

#[async_trait]
impl<F, I, O> Transformer for MapTransformer<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (I,);
  type OutputPorts = (O,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let f = self.f.clone();
    Box::pin(input.map(f))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<I>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<I> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<I> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<I>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<I>) -> ErrorContext<I> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: self.component_info().type_name,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "map_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

impl<F, I, O> ZeroCopyTransformer for MapTransformer<F, I, O>
where
  F: FnMut(I) -> O + Send + Clone + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + 'static,
  O: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Transforms an item using zero-copy semantics with `Cow`.
  ///
  /// For `MapTransformer`, this method handles both `Cow::Borrowed` and `Cow::Owned`
  /// inputs. Since mapping produces a new value, we always return `Cow::Owned`.
  ///
  /// # Zero-Copy Behavior
  ///
  /// - `Cow::Borrowed`: Clones the input before applying the transformation function
  /// - `Cow::Owned`: Uses the owned value directly (no additional clone)
  ///
  /// Both cases return `Cow::Owned` since the transformation produces a new value.
  fn transform_zero_copy<'a>(&mut self, input: Cow<'a, I>) -> Cow<'a, O> {
    // Extract the value from Cow (clone if Borrowed, move if Owned)
    let value = match input {
      Cow::Borrowed(borrowed) => borrowed.clone(), // Clone borrowed value
      Cow::Owned(owned) => owned,                  // Use owned value directly
    };

    // Apply the transformation function
    let transformed = (self.f)(value);

    // Always return Owned since transformation produces a new value
    Cow::Owned(transformed)
  }
}
