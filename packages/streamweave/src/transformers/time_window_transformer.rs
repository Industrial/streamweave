use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::collections::VecDeque;
use std::pin::Pin;
use std::time::Duration;
use tokio::time::interval;

/// A transformer that creates time-based windows of items from a stream.
///
/// This transformer groups consecutive items into windows based on time duration,
/// producing vectors of items as windows are created based on time intervals.
#[derive(Clone)]
pub struct TimeWindowTransformer<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  /// The duration of each window.
  pub duration: Duration,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: std::marker::PhantomData<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> TimeWindowTransformer<T> {
  /// Creates a new `TimeWindowTransformer` with the given window duration.
  ///
  /// # Arguments
  ///
  /// * `duration` - The duration of each window.
  pub fn new(duration: Duration) -> Self {
    Self {
      duration,
      config: TransformerConfig::<T>::default(),
      _phantom: std::marker::PhantomData,
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

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Input for TimeWindowTransformer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for TimeWindowTransformer<T> {
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}

#[async_trait]
impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Transformer for TimeWindowTransformer<T> {
  type InputPorts = (T,);
  type OutputPorts = (Vec<T>,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let duration = self.duration;
    Box::pin(async_stream::stream! {
      let mut window: VecDeque<T> = VecDeque::new();
      let mut input = input;
      let mut interval = interval(duration);

      // Skip the first tick (interval starts immediately)
      interval.tick().await;

      loop {
        tokio::select! {
          // Check if interval has ticked (time window expired)
          _ = interval.tick() => {
            // Time window expired, emit current window if it has items
            if !window.is_empty() {
              let window_vec: Vec<T> = window.iter().cloned().collect();
              yield window_vec;
              window.clear();
            }
          }
          // Check for new items from input stream
          item = input.next() => {
            match item {
              Some(item) => {
                window.push_back(item);
              }
              None => {
                // Stream ended, emit any remaining items as partial window
                if !window.is_empty() {
                  yield window.iter().cloned().collect::<Vec<_>>();
                }
                break;
              }
            }
          }
        }
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
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
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
        .unwrap_or_else(|| "time_window_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
