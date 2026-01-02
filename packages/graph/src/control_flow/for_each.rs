//! Transformer that expands collections into individual items (for-each loop).
//!
//! Takes items that are collections (e.g., `Vec<T>`) and expands them into
//! a stream of individual items. Uses zero-copy semantics where possible.

use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::{Input, Output, Transformer, TransformerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, StreamError};

/// Transformer that expands collections into individual items (for-each loop).
///
/// Takes items that are collections (e.g., `Vec<T>`) and expands them into
/// a stream of individual items. Uses zero-copy semantics where possible.
///
/// # Example
///
/// ```rust
/// use streamweave::graph::control_flow::ForEach;
/// use streamweave::graph::node::TransformerNode;
///
/// let for_each = ForEach::new(|item: &Vec<i32>| item.clone());
/// let node = TransformerNode::from_transformer(
///     "expand".to_string(),
///     for_each,
/// );
/// ```
pub struct ForEach<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  /// Function to extract collection from item
  #[allow(clippy::type_complexity)]
  extract_collection: Arc<dyn Fn(&T) -> Vec<T> + Send + Sync>,
  /// Configuration for the transformer
  config: TransformerConfig<T>,
}

impl<T> ForEach<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `ForEach` transformer.
  ///
  /// # Arguments
  ///
  /// * `extract_collection` - Function that extracts a collection from an item.
  ///   The collection will be expanded into individual items in the output stream.
  ///
  /// # Returns
  ///
  /// A new `ForEach` transformer instance.
  pub fn new<F>(extract_collection: F) -> Self
  where
    F: Fn(&T) -> Vec<T> + Send + Sync + 'static,
  {
    Self {
      extract_collection: Arc::new(extract_collection),
      config: TransformerConfig::default(),
    }
  }
}

impl<T> Input for ForEach<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for ForEach<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for ForEach<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let extract = Arc::clone(&self.extract_collection);
    Box::pin(input.flat_map(move |item| {
      let collection = extract(&item);
      futures::stream::iter(collection)
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
        .unwrap_or_else(|| "for_each".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
