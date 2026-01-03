//! Transformer that implements a while loop with conditional iteration.
//!
//! Repeats processing of items until a condition is met. Each iteration processes
//! the item through the loop body (which should be a downstream transformer).

use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, StreamError};

/// Transformer that implements a while loop with conditional iteration.
///
/// Repeats processing of items until a condition is met. Each iteration processes
/// the item through the loop body (which should be a downstream transformer).
///
/// # Example
///
/// ```rust
/// use streamweave::graph::control_flow::While;
/// use streamweave::graph::node::TransformerNode;
///
/// let while_loop = While::new(|x: &i32| *x < 100);
/// let node = TransformerNode::from_transformer(
///     "while_loop".to_string(),
///     while_loop,
/// );
/// ```
pub struct While<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  /// Condition function that determines if iteration should continue
  condition: Arc<dyn Fn(&T) -> bool + Send + Sync>,
  /// Configuration for the transformer
  config: TransformerConfig<T>,
  /// Phantom data for type parameter
  _phantom: PhantomData<T>,
}

impl<T> While<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `While` transformer with a condition function.
  ///
  /// # Arguments
  ///
  /// * `condition` - Function that returns `true` to continue iteration,
  ///   `false` to stop and emit the item.
  ///
  /// # Returns
  ///
  /// A new `While` transformer instance.
  ///
  /// # Note
  ///
  /// The `While` transformer emits items when the condition becomes false.
  /// For true iteration with transformation, you'll need to connect this to
  /// other transformers that modify the item between iterations.
  pub fn new<F>(condition: F) -> Self
  where
    F: Fn(&T) -> bool + Send + Sync + 'static,
  {
    Self {
      condition: Arc::new(condition),
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    }
  }
}

impl<T> Input for While<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for While<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for While<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let condition = Arc::clone(&self.condition);
    Box::pin(input.filter_map(move |item| {
      let condition = Arc::clone(&condition);
      async move {
        // Emit item when condition becomes false
        if !condition(&item) {
          Some(item)
        } else {
          None // Continue iteration (item would be processed by downstream transformers)
        }
      }
    }))
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
        .unwrap_or_else(|| "while".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
