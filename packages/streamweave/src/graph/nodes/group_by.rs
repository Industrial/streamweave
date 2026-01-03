//! Transformer that groups items by a key function.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

/// Transformer that groups items by a key function.
///
/// # Example
///
/// ```rust
/// use streamweave::graph::control_flow::GroupBy;
/// use streamweave::graph::node::TransformerNode;
///
/// let group_by = GroupBy::new(|item: &(String, i32)| item.0.clone());
/// let node = TransformerNode::from_transformer(
///     "group".to_string(),
///     group_by,
/// );
/// ```
pub struct GroupBy<
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
> {
  /// Function to extract key from item
  key_fn: Arc<dyn Fn(&T) -> K + Send + Sync>,
  /// Configuration for the transformer
  config: TransformerConfig<T>,
  /// Phantom data for type parameters
  _phantom: PhantomData<(T, K)>,
}

impl<T, K> GroupBy<T, K>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
{
  /// Creates a new `GroupBy` transformer.
  ///
  /// # Arguments
  ///
  /// * `key_fn` - Function that extracts a key from each item
  ///
  /// # Returns
  ///
  /// A new `GroupBy` transformer instance.
  pub fn new<F>(key_fn: F) -> Self
  where
    F: Fn(&T) -> K + Send + Sync + 'static,
  {
    Self {
      key_fn: Arc::new(key_fn),
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    }
  }
}

impl<T, K> Input for GroupBy<T, K>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T, K> Output for GroupBy<T, K>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
{
  type Output = (K, Vec<T>);
  type OutputStream = Pin<Box<dyn Stream<Item = (K, Vec<T>)> + Send>>;
}

#[async_trait]
impl<T, K> Transformer for GroupBy<T, K>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = ((K, Vec<T>),);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let key_fn = Arc::clone(&self.key_fn);
    let mut groups: HashMap<K, Vec<T>> = HashMap::new();

    let mut stream = input;
    while let Some(item) = stream.next().await {
      let key = (key_fn)(&item);
      groups.entry(key).or_default().push(item);
    }

    // Emit all groups
    Box::pin(futures::stream::iter(groups))
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
      crate::error::ErrorStrategy::Stop => ErrorAction::Stop,
      crate::error::ErrorStrategy::Skip => ErrorAction::Skip,
      crate::error::ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      crate::error::ErrorStrategy::Custom(ref handler) => handler(error),
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
        .unwrap_or_else(|| "group_by".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
