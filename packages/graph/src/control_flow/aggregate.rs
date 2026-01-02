//! Transformer that aggregates items using an aggregator function.
//!
//! Includes common aggregator implementations (Sum, Count, Min, Max).

use async_stream::stream;
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::{Input, Output, Transformer, TransformerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, StreamError};

/// Trait for aggregating items into a single value.
pub trait Aggregator<T, A>: Send + Sync {
  /// Initial value for the accumulator.
  fn init(&self) -> A;
  /// Accumulates an item into the accumulator.
  fn accumulate(&self, acc: &mut A, item: &T);
  /// Finalizes the accumulator into the result.
  fn finalize(&self, acc: A) -> A;
}

/// Transformer that aggregates items using an aggregator function.
///
/// # Example
///
/// ```rust
/// use streamweave::graph::control_flow::{Aggregate, Aggregator, SumAggregator};
/// use streamweave::graph::node::TransformerNode;
///
/// let aggregate = Aggregate::new(SumAggregator, Some(10)); // Window size of 10
/// let node = TransformerNode::from_transformer(
///     "sum".to_string(),
///     aggregate,
/// );
/// ```
pub struct Aggregate<
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  A: Send + Sync + Clone + 'static,
> {
  /// Aggregator implementation
  aggregator: Arc<dyn Aggregator<T, A>>,
  /// Optional window size (number of items to aggregate before emitting)
  window_size: Option<usize>,
  /// Configuration for the transformer
  config: TransformerConfig<T>,
  /// Phantom data for type parameters
  _phantom: PhantomData<(T, A)>,
}

impl<T, A> Aggregate<T, A>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  A: Send + Sync + Clone + 'static,
{
  /// Creates a new `Aggregate` transformer.
  ///
  /// # Arguments
  ///
  /// * `aggregator` - The aggregator implementation
  /// * `window_size` - Optional window size. If `Some(n)`, aggregates `n` items
  ///   before emitting. If `None`, aggregates all items until stream ends.
  ///
  /// # Returns
  ///
  /// A new `Aggregate` transformer instance.
  pub fn new(aggregator: impl Aggregator<T, A> + 'static, window_size: Option<usize>) -> Self {
    Self {
      aggregator: Arc::new(aggregator),
      window_size,
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    }
  }
}

impl<T, A> Input for Aggregate<T, A>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  A: Send + Sync + Clone + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T, A> Output for Aggregate<T, A>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  A: Send + Sync + Clone + 'static,
{
  type Output = A;
  type OutputStream = Pin<Box<dyn Stream<Item = A> + Send>>;
}

#[async_trait]
impl<T, A> Transformer for Aggregate<T, A>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  A: Send + Sync + Clone + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (A,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let aggregator = Arc::clone(&self.aggregator);
    let window_size = self.window_size;

    Box::pin(stream! {
      let mut acc = aggregator.init();
      let mut count = 0;

      let mut stream = input;
      while let Some(item) = stream.next().await {
        aggregator.accumulate(&mut acc, &item);
        count += 1;

        if let Some(window) = window_size
          && count >= window
        {
          yield aggregator.finalize(acc.clone());
          acc = aggregator.init();
          count = 0;
        }
      }

      // Emit final aggregate if we have items and no window size
      if count > 0 && window_size.is_none() {
        yield aggregator.finalize(acc);
      }
    })
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Self::Input> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction {
    match self.config.error_strategy() {
      streamweave_error::ErrorStrategy::Stop => ErrorAction::Stop,
      streamweave_error::ErrorStrategy::Skip => ErrorAction::Skip,
      streamweave_error::ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      streamweave_error::ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Self::Input>) -> ErrorContext<Self::Input> {
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
        .name()
        .clone()
        .unwrap_or_else(|| "aggregate".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

/// Aggregator that sums numeric values.
pub struct SumAggregator;

impl<T> Aggregator<T, T> for SumAggregator
where
  T: std::ops::Add<Output = T> + Clone + Default + Send + Sync,
{
  fn init(&self) -> T {
    T::default()
  }

  fn accumulate(&self, acc: &mut T, item: &T) {
    *acc = acc.clone() + item.clone();
  }

  fn finalize(&self, acc: T) -> T {
    acc
  }
}

/// Aggregator that counts items.
pub struct CountAggregator;

impl<T> Aggregator<T, usize> for CountAggregator {
  fn init(&self) -> usize {
    0
  }

  fn accumulate(&self, acc: &mut usize, _item: &T) {
    *acc += 1;
  }

  fn finalize(&self, acc: usize) -> usize {
    acc
  }
}

/// Aggregator that finds the minimum value.
pub struct MinAggregator;

impl<T> Aggregator<T, Option<T>> for MinAggregator
where
  T: PartialOrd + Clone + Send + Sync,
{
  fn init(&self) -> Option<T> {
    None
  }

  fn accumulate(&self, acc: &mut Option<T>, item: &T) {
    match acc {
      None => *acc = Some(item.clone()),
      Some(current) => {
        if item < current {
          *acc = Some(item.clone());
        }
      }
    }
  }

  fn finalize(&self, acc: Option<T>) -> Option<T> {
    acc
  }
}

/// Aggregator that finds the maximum value.
pub struct MaxAggregator;

impl<T> Aggregator<T, Option<T>> for MaxAggregator
where
  T: PartialOrd + Clone + Send + Sync,
{
  fn init(&self) -> Option<T> {
    None
  }

  fn accumulate(&self, acc: &mut Option<T>, item: &T) {
    match acc {
      None => *acc = Some(item.clone()),
      Some(current) => {
        if item > current {
          *acc = Some(item.clone());
        }
      }
    }
  }

  fn finalize(&self, acc: Option<T>) -> Option<T> {
    acc
  }
}
