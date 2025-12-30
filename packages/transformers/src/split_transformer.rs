//! Split transformer for StreamWeave

use async_trait::async_trait;
use futures::{Stream, StreamExt, stream};
use std::marker::PhantomData;
use std::pin::Pin;
use streamweave::{Input, Output, Transformer, TransformerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

/// A transformer that splits a stream of items into groups based on a predicate.
///
/// This transformer groups consecutive items where the predicate returns `true`
/// into separate vectors, effectively splitting the stream at points where the
/// predicate returns `false`.
#[derive(Clone)]
pub struct SplitTransformer<F, T>
where
  F: Send + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The predicate function used to determine split points.
  pub predicate: F,
  /// Phantom data to track the item type parameter.
  pub _phantom: PhantomData<T>,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<Vec<T>>,
}

impl<F, T> SplitTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `SplitTransformer` with the given predicate.
  ///
  /// # Arguments
  ///
  /// * `predicate` - The function to use for determining split points.
  pub fn new(predicate: F) -> Self {
    Self {
      predicate,
      _phantom: PhantomData,
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Vec<T>>) -> Self {
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

impl<F, T> Input for SplitTransformer<F, T>
where
  F: Send + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = Vec<T>;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

impl<F, T> Output for SplitTransformer<F, T>
where
  F: Send + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl<F, T> Transformer for SplitTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (Vec<T>,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let predicate = self.predicate.clone();

    Box::pin(futures::stream::unfold(
      (input, predicate),
      |(mut input, mut pred)| async move {
        if let Some(items) = input.next().await {
          let mut current_chunk = Vec::new();
          let mut chunks = Vec::new();

          for item in items {
            if pred(&item) && !current_chunk.is_empty() {
              chunks.push(std::mem::take(&mut current_chunk));
            }
            current_chunk.push(item);
          }

          if !current_chunk.is_empty() {
            chunks.push(current_chunk);
          }

          if chunks.is_empty() {
            Some((Vec::new(), (input, pred)))
          } else {
            let mut iter = chunks.into_iter();
            let first = iter.next().unwrap();
            Some((
              first,
              (
                Box::pin(stream::iter(iter).chain(input))
                  as Pin<Box<dyn Stream<Item = Vec<T>> + Send>>,
                pred,
              ),
            ))
          }
        } else {
          None
        }
      },
    ))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Vec<T>>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Vec<T>> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Vec<T>> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Vec<T>>) -> ErrorAction {
    match &self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      ErrorStrategy::Custom(handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Vec<T>>) -> ErrorContext<Vec<T>> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "split_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "split_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
