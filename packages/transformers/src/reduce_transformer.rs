//! Reduce transformer for StreamWeave

use async_stream::stream;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::borrow::Cow;
use std::marker::PhantomData;
use std::pin::Pin;
use streamweave::{Input, Output, Transformer, TransformerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_graph::ZeroCopyTransformer;

/// A transformer that reduces a stream of items into a single accumulated value.
///
/// This transformer applies a reducer function to each item in the stream along with
/// an accumulator, producing a final accumulated value.
pub struct ReduceTransformer<T, Acc, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Acc: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(Acc, T) -> Acc + Send + Clone + 'static,
{
  /// The initial accumulator value.
  pub accumulator: Acc,
  /// The reducer function that combines the accumulator with each item.
  pub reducer: F,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T, Acc, F> ReduceTransformer<T, Acc, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Acc: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(Acc, T) -> Acc + Send + Clone + 'static,
{
  /// Creates a new `ReduceTransformer` with the given initial accumulator and reducer function.
  ///
  /// # Arguments
  ///
  /// * `initial` - The initial value for the accumulator.
  /// * `reducer` - The function that combines the accumulator with each item.
  pub fn new(initial: Acc, reducer: F) -> Self {
    Self {
      accumulator: initial,
      reducer,
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    }
  }

  /// Sets the error handling strategy for this transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config = self.config.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this transformer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config = self.config.with_name(name);
    self
  }
}

impl<T, Acc, F> Input for ReduceTransformer<T, Acc, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Acc: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(Acc, T) -> Acc + Send + Clone + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T, Acc, F> Output for ReduceTransformer<T, Acc, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Acc: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(Acc, T) -> Acc + Send + Clone + 'static,
{
  type Output = Acc;
  type OutputStream = Pin<Box<dyn Stream<Item = Acc> + Send>>;
}

#[async_trait]
impl<T, Acc, F> Transformer for ReduceTransformer<T, Acc, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Acc: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(Acc, T) -> Acc + Send + Clone + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (Acc,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let mut reducer = self.reducer.clone();
    let initial = self.accumulator.clone();
    Box::pin(stream! {
        let mut acc = initial;
        let mut input = input;

        while let Some(item) = input.next().await {
            acc = reducer(acc.clone(), item);
            yield acc.clone();
        }
    })
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
    match &self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      ErrorStrategy::Custom(handler) => handler(error),
      _ => ErrorAction::Stop,
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
        .unwrap_or_else(|| "reduce_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "reduce_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

impl<T, Acc, F> ZeroCopyTransformer for ReduceTransformer<T, Acc, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Acc: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(Acc, T) -> Acc + Send + Clone + 'static,
{
  /// Transforms an item using zero-copy semantics with `Cow`.
  ///
  /// For `ReduceTransformer`, this method handles both `Cow::Borrowed` and `Cow::Owned`
  /// inputs. The input value is extracted from the Cow (cloned if Borrowed, moved if Owned),
  /// then combined with the current accumulator using the reducer function. The updated
  /// accumulator is returned as `Cow::Owned`.
  ///
  /// # Zero-Copy Behavior
  ///
  /// - `Cow::Borrowed`: Clones the input value before applying reducer (necessary for reduction)
  /// - `Cow::Owned`: Uses the owned value directly (no additional clone of input)
  ///
  /// Both cases return `Cow::Owned` since the accumulator is always owned by the transformer.
  /// The accumulator state is updated in-place, maintaining correct reduction semantics.
  ///
  /// # State Management
  ///
  /// This method mutates the internal accumulator state. Each call updates the accumulator
  /// with the new item, enabling incremental reduction while preserving zero-copy semantics
  /// for the input value extraction.
  fn transform_zero_copy<'a>(&mut self, input: Cow<'a, T>) -> Cow<'a, Acc> {
    // Extract the value from Cow (clone if Borrowed, move if Owned)
    let value = match input {
      Cow::Borrowed(borrowed) => borrowed.clone(), // Clone borrowed value
      Cow::Owned(owned) => owned,                 // Use owned value directly
    };

    // Apply the reducer function: new_acc = reducer(current_acc, value)
    let mut reducer = self.reducer.clone();
    let new_accumulator = reducer(self.accumulator.clone(), value);

    // Update the accumulator state
    self.accumulator = new_accumulator.clone();

    // Always return Owned since accumulator is owned by transformer
    Cow::Owned(new_accumulator)
  }
}
