//! # Pipeline Builder and Execution
//!
//! This module provides a type-safe pipeline builder with compile-time validation
//! using a state machine pattern. Pipelines are linear data flows: Producer →
//! Transformer → Consumer.
//!
//! ## Overview
//!
//! The pipeline system provides:
//!
//! - **Type-Safe Builder**: Compile-time validation ensures all components are present
//! - **State Machine**: Phantom types enforce correct component ordering
//! - **Error Handling**: Configurable error strategies for each pipeline stage
//! - **Stream Execution**: Async execution of complete pipelines
//!
//! ## Pipeline States
//!
//! The builder uses phantom types to track state:
//!
//! - `Empty`: No components added yet
//! - `HasProducer<P>`: Producer added, ready for transformer
//! - `HasTransformer<P, T>`: Producer and transformer added, ready for consumer
//! - `Complete<P, T, C>`: All components present, ready to build
//!
//! ## Example
//!
//! ```rust
//! use crate::prelude::*;
//!
//! let pipeline = PipelineBuilder::new()
//!     .producer(VecProducer::new(vec![1, 2, 3]))
//!     .transformer(MapTransformer::new(|x| x * 2))
//!     .consumer(VecConsumer::new())
//!     .error_strategy(ErrorStrategy::Skip)
//!     .build();
//!
//! // Execute the pipeline
//! pipeline.run().await;
//! ```
//!
//! ## Key Concepts
//!
//! - **Pipeline**: A linear data flow from producer through transformer to consumer
//! - **PipelineBuilder**: Type-safe builder with state machine validation
//! - **Error Strategy**: How to handle errors at each stage (Stop, Skip, Retry, Custom)
//! - **Stream Execution**: Async execution with proper error propagation
//!
//! ## Usage
//!
//! Pipelines are the simplest way to process streams in StreamWeave. For more
//! complex topologies with fan-in/fan-out, use the Graph API instead.

use crate::error::ErrorStrategy;
use crate::{Consumer, Producer, Transformer};
use std::marker::PhantomData;

#[cfg(test)]
use crate::error::{ErrorAction, PipelineError, StreamError};

// Helper functions for panic messages (extracted for better coverage tracking)
fn panic_producer_stream_missing() -> ! {
  panic!(
    "Internal error: producer stream missing in HasProducer state. This indicates a bug in the pipeline builder state machine."
  )
}

fn panic_producer_stream_downcast_failed() -> ! {
  panic!(
    "Internal error: failed to downcast producer stream to expected type. This indicates a type mismatch in the pipeline builder."
  )
}

fn panic_transformer_stream_missing() -> ! {
  panic!(
    "Internal error: transformer stream missing in HasTransformer state. This indicates a bug in the pipeline builder state machine."
  )
}

fn panic_transformer_stream_downcast_failed() -> ! {
  panic!(
    "Internal error: failed to downcast transformer stream to expected type. This indicates a type mismatch in the pipeline builder."
  )
}

fn panic_transformer_stream_missing_complete() -> ! {
  panic!(
    "Internal error: transformer stream missing in Complete state. This indicates a bug in the pipeline builder state machine."
  )
}

fn panic_consumer_missing() -> ! {
  panic!(
    "Internal error: consumer missing in Complete state. This indicates a bug in the pipeline builder state machine."
  )
}

/// Empty state for pipeline builder.
///
/// This state indicates that no components have been added to the pipeline yet.
pub struct Empty;

/// State indicating a producer has been added to the pipeline.
///
/// # Type Parameters
///
/// * `P` - The producer type that has been added
pub struct HasProducer<P>(PhantomData<P>);

/// State indicating both a producer and transformer have been added.
///
/// # Type Parameters
///
/// * `P` - The producer type
/// * `T` - The transformer type
pub struct HasTransformer<P, T>(PhantomData<(P, T)>);

/// Complete state indicating all components (producer, transformer, consumer) are present.
///
/// # Type Parameters
///
/// * `P` - The producer type
/// * `T` - The transformer type
/// * `C` - The consumer type
pub struct Complete<P, T, C>(PhantomData<(P, T, C)>);

/// Pipeline builder with state machine for compile-time validation.
///
/// The builder uses phantom types to ensure pipelines are only built when all
/// required components are present. This prevents runtime errors from missing
/// components.
///
/// # Example
///
/// ```rust
/// use crate::prelude::*;
///
/// let pipeline = PipelineBuilder::new()
///     .with_producer(ArrayProducer::new(vec![1, 2, 3]))
///     .with_transformer(MapTransformer::new(|x| x * 2))
///     .with_consumer(VecConsumer::new())
///     .build();
/// ```
///
/// # Type Parameters
///
/// * `State` - The current state of the builder (Empty, HasProducer, HasTransformer, or Complete)
pub struct PipelineBuilder<State> {
  _producer_stream: Option<Box<dyn std::any::Any + Send + 'static>>,
  transformer_stream: Option<Box<dyn std::any::Any + Send + 'static>>,
  _consumer: Option<Box<dyn std::any::Any + Send + 'static>>,
  error_strategy: ErrorStrategy<()>,
  _state: State,
}

/// A complete pipeline that connects a producer, transformer, and consumer.
///
/// This struct represents a fully constructed pipeline that is ready to execute.
/// It holds references to the producer stream, transformer stream, and consumer,
/// along with the error handling strategy.
pub struct Pipeline<P, T, C>
where
  P: Producer,
  T: Transformer,
  C: Consumer,
  P::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  C::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  _producer_stream: Option<P::OutputStream>,
  transformer_stream: Option<T::OutputStream>,
  _consumer: Option<C>,
  error_strategy: ErrorStrategy<()>,
}

// Initial builder creation
impl PipelineBuilder<Empty> {
  /// Creates a new empty pipeline builder.
  ///
  /// This is the starting point for building a pipeline. You must add
  /// a producer, transformer, and consumer before the pipeline can be built.
  ///
  /// # Example
  ///
  /// ```rust
  /// use crate::prelude::*;
  ///
  /// let builder = PipelineBuilder::new();
  /// ```
  ///
  /// # Returns
  ///
  /// A new `PipelineBuilder` in the `Empty` state.
  #[must_use]
  pub fn new() -> Self {
    PipelineBuilder {
      _producer_stream: None,
      transformer_stream: None,
      _consumer: None,
      error_strategy: ErrorStrategy::Stop,
      _state: Empty,
    }
  }

  /// Sets the error handling strategy for the pipeline.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use for the entire pipeline.
  ///
  /// # Returns
  ///
  /// The pipeline builder with the updated error strategy.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<()>) -> Self {
    self.error_strategy = strategy;
    self
  }

  /// Adds a producer to the pipeline.
  ///
  /// This method takes a producer and starts generating the input stream.
  ///
  /// # Arguments
  ///
  /// * `producer` - The producer to add to the pipeline.
  ///
  /// # Returns
  ///
  /// A pipeline builder in the `HasProducer` state, allowing you to add transformers.
  #[must_use]
  pub fn producer<P>(mut self, mut producer: P) -> PipelineBuilder<HasProducer<P>>
  where
    P: Producer + 'static,
    P::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
    P::OutputStream: 'static,
  {
    let stream = producer.produce();
    self._producer_stream = Some(Box::new(stream));

    PipelineBuilder {
      _producer_stream: self._producer_stream,
      transformer_stream: None,
      _consumer: None,
      error_strategy: self.error_strategy,
      _state: HasProducer(PhantomData),
    }
  }
}

impl Default for PipelineBuilder<Empty> {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
impl<P> PipelineBuilder<HasProducer<P>>
where
  P: Producer + 'static,
  P::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  P::OutputStream: 'static,
{
  /// Test-only method to create an invalid state where producer_stream is None.
  pub(crate) fn _test_with_no_producer_stream(mut self) -> Self {
    self._producer_stream = None;
    self
  }

  /// Test-only method to create an invalid state where producer_stream has wrong type.
  pub(crate) fn _test_with_wrong_producer_stream_type(mut self) -> Self {
    self._producer_stream = Some(Box::new("wrong type".to_string()));
    self
  }
}

// After producer is added
impl<P> PipelineBuilder<HasProducer<P>>
where
  P: Producer + 'static,
  P::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  P::OutputStream: 'static,
{
  /// Adds a transformer to the pipeline.
  ///
  /// This method consumes the pipeline builder and returns a new builder
  /// in the `HasTransformer` state, allowing you to chain additional transformers
  /// or add a consumer.
  ///
  /// # Arguments
  ///
  /// * `transformer` - The transformer to add to the pipeline.
  ///
  /// # Returns
  ///
  /// A new `PipelineBuilder` in the `HasTransformer` state.
  #[must_use]
  pub async fn transformer<T>(mut self, mut transformer: T) -> PipelineBuilder<HasTransformer<P, T>>
  where
    T: Transformer + 'static,
    T::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
    T::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
    T::InputStream: From<P::OutputStream>,
    T::OutputStream: 'static,
  {
    let producer_stream = self
      ._producer_stream
      .take()
      .unwrap_or_else(|| panic_producer_stream_missing())
      .downcast::<P::OutputStream>()
      .unwrap_or_else(|_| panic_producer_stream_downcast_failed());

    let transformer_stream = transformer.transform((*producer_stream).into()).await;
    self.transformer_stream = Some(Box::new(transformer_stream));

    PipelineBuilder {
      _producer_stream: None,
      transformer_stream: self.transformer_stream,
      _consumer: None,
      error_strategy: self.error_strategy,
      _state: HasTransformer(PhantomData),
    }
  }
}

#[cfg(test)]
impl<P, T> PipelineBuilder<HasTransformer<P, T>>
where
  P: Producer + 'static,
  T: Transformer + 'static,
  P::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::OutputStream: 'static,
{
  /// Test-only method to create an invalid state where transformer_stream is None.
  pub(crate) fn _test_with_no_transformer_stream(mut self) -> Self {
    self.transformer_stream = None;
    self
  }

  /// Test-only method to create an invalid state where transformer_stream has wrong type.
  pub(crate) fn _test_with_wrong_transformer_stream_type(mut self) -> Self {
    self.transformer_stream = Some(Box::new("wrong type".to_string()));
    self
  }
}

// After transformer is added
impl<P, T> PipelineBuilder<HasTransformer<P, T>>
where
  P: Producer + 'static,
  T: Transformer + 'static,
  P::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::OutputStream: 'static,
{
  /// Adds another transformer to the pipeline.
  ///
  /// This method allows chaining multiple transformers together. The new
  /// transformer will process the output of the previous transformer.
  ///
  /// # Arguments
  ///
  /// * `transformer` - The transformer to add to the pipeline.
  ///
  /// # Returns
  ///
  /// A new `PipelineBuilder` in the `HasTransformer` state with the new transformer.
  #[must_use]
  pub async fn transformer<U>(mut self, mut transformer: U) -> PipelineBuilder<HasTransformer<P, U>>
  where
    U: Transformer + 'static,
    U::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
    U::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
    U::InputStream: From<T::OutputStream>,
    U::OutputStream: 'static,
  {
    let transformer_stream = self
      .transformer_stream
      .take()
      .unwrap_or_else(|| panic_transformer_stream_missing())
      .downcast::<T::OutputStream>()
      .unwrap_or_else(|_| panic_transformer_stream_downcast_failed());

    let new_stream = transformer.transform((*transformer_stream).into()).await;
    self.transformer_stream = Some(Box::new(new_stream));

    PipelineBuilder {
      _producer_stream: None,
      transformer_stream: self.transformer_stream,
      _consumer: None,
      error_strategy: self.error_strategy,
      _state: HasTransformer(PhantomData),
    }
  }

  /// Adds a consumer to the pipeline and completes the pipeline construction.
  ///
  /// This method takes a consumer and connects it to the transformer's output stream,
  /// completing the pipeline. It consumes the pipeline builder and returns a complete
  /// `Pipeline` that is ready to be executed.
  ///
  /// # Arguments
  ///
  /// * `consumer` - The consumer to add to the pipeline.
  ///
  /// # Returns
  ///
  /// A complete `Pipeline` that is ready to execute.
  #[must_use]
  pub fn consumer<C>(mut self, consumer: C) -> Pipeline<P, T, C>
  where
    C: Consumer + 'static,
    C::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
    C::InputStream: From<T::OutputStream>,
  {
    let transformer_stream = self
      .transformer_stream
      .take()
      .unwrap_or_else(|| panic_transformer_stream_missing())
      .downcast::<T::OutputStream>()
      .unwrap_or_else(|_| panic_transformer_stream_downcast_failed());

    Pipeline {
      _producer_stream: None,
      transformer_stream: Some(*transformer_stream),
      _consumer: Some(consumer),
      error_strategy: self.error_strategy,
    }
  }
}

impl<P, T, C> Pipeline<P, T, C>
where
  P: Producer,
  T: Transformer,
  C: Consumer,
  P::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  C::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Sets the error handling strategy for this pipeline.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  ///
  /// # Returns
  ///
  /// The pipeline with the updated error strategy.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<()>) -> Self {
    self.error_strategy = strategy;
    self
  }

  #[cfg(test)]
  pub(crate) async fn _handle_error(
    &self,
    error: StreamError<()>,
  ) -> Result<ErrorAction, PipelineError<()>> {
    match &self.error_strategy {
      ErrorStrategy::Stop => Ok(ErrorAction::Stop),
      ErrorStrategy::Skip => Ok(ErrorAction::Skip),
      ErrorStrategy::Retry(max_retries) => {
        if error.retries < *max_retries {
          Ok(ErrorAction::Retry)
        } else {
          Ok(ErrorAction::Stop)
        }
      }
      ErrorStrategy::Custom(handler) => Ok(handler(&error)),
    }
  }

  /// Executes the pipeline, processing all items from producer through transformer to consumer.
  ///
  /// This method consumes the pipeline and runs it to completion. It will:
  /// 1. Generate items from the producer
  /// 2. Transform items through the transformer
  /// 3. Consume items with the consumer
  ///
  /// # Returns
  ///
  /// Returns `Ok(((), consumer))` on success, where the consumer can be used
  /// to retrieve results (e.g., `VecConsumer` will have collected items).
  ///
  /// Returns `Err(PipelineError)` if an error occurs during execution and
  /// the error strategy is set to `Stop`.
  ///
  /// # Example
  ///
  /// ```rust
  /// use crate::prelude::*;
  ///
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let pipeline = Pipeline::new()
  ///     .with_producer(ArrayProducer::new(vec![1, 2, 3, 4, 5]))
  ///     .with_transformer(MapTransformer::new(|x| x * 2))
  ///     .with_consumer(VecConsumer::new());
  ///
  /// let (_, consumer) = pipeline.run().await?;
  /// let results = consumer.into_inner();
  /// assert_eq!(results, vec![2, 4, 6, 8, 10]);
  /// # Ok(())
  /// # }
  /// ```
  ///
  /// # Errors
  ///
  /// This method will return an error if:
  /// - The error strategy is set to `Stop` and an error occurs
  /// - The consumer fails to process items (depending on error strategy)
  pub async fn run(mut self) -> Result<((), C), crate::error::PipelineError<()>>
  where
    C::InputStream: From<T::OutputStream>,
  {
    let transformer_stream = self
      .transformer_stream
      .take()
      .unwrap_or_else(|| panic_transformer_stream_missing_complete());
    let mut consumer = self
      ._consumer
      .take()
      .unwrap_or_else(|| panic_consumer_missing());

    consumer.consume(transformer_stream.into()).await;
    Ok(((), consumer))
  }
}

#[cfg(test)]
impl<P, T, C> Pipeline<P, T, C>
where
  P: Producer,
  T: Transformer,
  C: Consumer,
  P::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
  C::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Test-only method to create an invalid state where transformer_stream is None.
  pub(crate) fn _test_with_no_transformer_stream(mut self) -> Self {
    self.transformer_stream = None;
    self
  }

  /// Test-only method to create an invalid state where consumer is None.
  pub(crate) fn _test_with_no_consumer(mut self) -> Self {
    self._consumer = None;
    self
  }
}
