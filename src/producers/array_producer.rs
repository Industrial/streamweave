//! Array producer for emitting items from compile-time-sized arrays.
//!
//! This module provides [`ArrayProducer<T, N>`], a producer that emits items from
//! a fixed-size array. The array size is determined at compile time via the const
//! generic parameter `N`, enabling efficient zero-allocation iteration over array
//! elements.
//!
//! # Overview
//!
//! [`ArrayProducer`] is useful for producing streams from compile-time-known arrays.
//! Unlike `VecProducer` which uses runtime-sized vectors, `ArrayProducer` uses
//! compile-time-sized arrays, enabling compiler optimizations and zero-overhead
//! iteration. This is ideal for small, fixed-size datasets that are known at
//! compile time.
//!
//! # Key Concepts
//!
//! - **Compile-Time Size**: Array size `N` is determined at compile time
//! - **Const Generics**: Uses const generics for type-safe array size
//! - **Zero Allocation**: Efficient iteration without heap allocations
//! - **Fixed Size**: Array size cannot change at runtime
//! - **Simple Producer**: Direct array-to-stream conversion
//!
//! # Core Types
//!
//! - **[`ArrayProducer<T, N>`]**: Producer that emits items from an array of size `N`
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::producers::ArrayProducer;
//! use streamweave::PipelineBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a producer from a fixed-size array
//! let array = [1, 2, 3, 4, 5];
//! let producer = ArrayProducer::new(array);
//!
//! // Produces: 1, 2, 3, 4, 5
//! # Ok(())
//! # }
//! ```
//!
//! ## From Slice (at compile time)
//!
//! ```rust
//! use streamweave::producers::ArrayProducer;
//!
//! // Create from a slice if length matches
//! let slice = &[10, 20, 30];
//! if let Some(producer) = ArrayProducer::<i32, 3>::from_slice(slice) {
//!     // Producer created successfully
//! }
//! ```
//!
//! # Design Decisions
//!
//! - **Const Generics**: Uses const generics for compile-time array size
//! - **Fixed Size**: Array size must be known at compile time
//! - **Efficiency**: Zero-allocation iteration over array elements
//! - **Type Safety**: Compile-time guarantees about array size
//! - **Simple API**: Direct array-to-stream conversion
//!
//! # Integration with StreamWeave
//!
//! [`ArrayProducer`] implements the [`Producer`] trait and can be used in any
//! StreamWeave pipeline or graph. It supports the standard error handling strategies
//! and configuration options provided by [`ProducerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Output, Producer, ProducerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// A producer that emits items from a fixed-size array.
///
/// This producer iterates over the elements of an internal array and
/// emits them as a stream. The array size is determined at compile time
/// via the const generic parameter `N`.
pub struct ArrayProducer<T: Send + Sync + 'static + Clone + std::fmt::Debug, const N: usize> {
  /// The array containing the data to be produced.
  pub array: [T; N],
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: std::marker::PhantomData<T>,
}

impl<T: Send + Sync + 'static + Clone + std::fmt::Debug, const N: usize> ArrayProducer<T, N> {
  /// Creates a new `ArrayProducer` with the given array.
  ///
  /// # Arguments
  ///
  /// * `array` - The array whose elements will be produced.
  pub fn new(array: [T; N]) -> Self {
    Self {
      array,
      config: ProducerConfig::<T>::default(),
      _phantom: std::marker::PhantomData,
    }
  }

  /// Sets the error handling strategy for this producer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this producer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Creates an `ArrayProducer` from a slice, if the slice length matches `N`.
  ///
  /// # Arguments
  ///
  /// * `slice` - The slice to convert to an array.
  ///
  /// # Returns
  ///
  /// Returns `Some(ArrayProducer)` if the slice length equals `N`, otherwise `None`.
  ///
  /// # Safety
  ///
  /// This function uses unsafe code to initialize the array from the slice.
  /// The caller must ensure that `T: Copy` is satisfied.
  pub fn from_slice(slice: &[T]) -> Option<Self>
  where
    T: Copy,
  {
    if slice.len() != N {
      return None;
    }
    let mut arr = std::mem::MaybeUninit::<[T; N]>::uninit();
    let ptr = arr.as_mut_ptr() as *mut T;
    unsafe {
      for (i, &item) in slice.iter().enumerate() {
        ptr.add(i).write(item);
      }
      Some(Self::new(arr.assume_init()))
    }
  }

  /// Returns the length of the array (always `N`).
  pub fn len(&self) -> usize {
    N
  }

  /// Returns `true` if the array is empty (i.e., `N == 0`).
  pub fn is_empty(&self) -> bool {
    N == 0
  }
}

// Trait implementations for ArrayProducer

impl<T: Send + Sync + 'static + Clone + std::fmt::Debug, const N: usize> Output
  for ArrayProducer<T, N>
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T: Send + Sync + 'static + Clone + std::fmt::Debug, const N: usize> Producer
  for ArrayProducer<T, N>
{
  type OutputPorts = (T,);

  fn produce(&mut self) -> Self::OutputStream {
    let array = self.array.clone();
    Box::pin(futures::stream::iter(array))
  }

  fn set_config_impl(&mut self, config: ProducerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) => {
        if error.retries < n {
          ErrorAction::Retry
        } else {
          ErrorAction::Stop
        }
      }
      ErrorStrategy::Custom(ref handler) => handler(error),
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "array_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "array_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
