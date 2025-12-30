use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::collections::VecDeque;
use std::pin::Pin;
use streamweave::{Input, Output, Transformer, TransformerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

/// A transformer that creates sliding windows of items from a stream.
///
/// This transformer groups consecutive items into windows of a specified size,
/// producing vectors of items as the window slides over the input stream.
#[derive(Clone)]
pub struct WindowTransformer<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  /// The size of each window.
  pub size: usize,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: std::marker::PhantomData<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> WindowTransformer<T> {
  /// Creates a new `WindowTransformer` with the given window size.
  ///
  /// # Arguments
  ///
  /// * `size` - The size of each window.
  pub fn new(size: usize) -> Self {
    Self {
      size,
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

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Input for WindowTransformer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for WindowTransformer<T> {
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}

#[async_trait]
impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Transformer for WindowTransformer<T> {
  type InputPorts = (T,);
  type OutputPorts = (Vec<T>,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let size = self.size;
    Box::pin(async_stream::stream! {
      let mut window: VecDeque<T> = VecDeque::with_capacity(size);
      let mut input = input;

      while let Some(item) = input.next().await {
        window.push_back(item);

        if window.len() == size {
          // Efficiently yield the current window as a Vec
          let window_vec: Vec<T> = window.iter().cloned().collect();
          yield window_vec;
          // Slide the window by removing the oldest item
          window.pop_front();
        }
      }

      // Emit any remaining items as a partial window
      if !window.is_empty() {
        yield window.iter().cloned().collect::<Vec<_>>();
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
        .unwrap_or_else(|| "window_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
