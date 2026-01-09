//! Trace node for StreamWeave graphs
//!
//! Logs/debugs packet flow by tracing items as they pass through. Items are
//! emitted unchanged but logged for debugging purposes.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::{TraceLevel, TraceTransformer};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that traces/logs items as they pass through without modifying them.
///
/// This node wraps `TraceTransformer` for use in graphs. It logs each item that
/// passes through and emits it unchanged. The logging level can be controlled
/// via the `log_level` parameter.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{Trace, TransformerNode};
/// use crate::transformers::TraceLevel;
///
/// let trace = Trace::new();
/// let node = TransformerNode::from_transformer(
///     "trace".to_string(),
///     trace,
/// );
/// ```
pub struct Trace<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying trace transformer
  transformer: TraceTransformer<T>,
}

impl<T> Trace<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Trace` node with default debug logging.
  pub fn new() -> Self {
    Self {
      transformer: TraceTransformer::new(),
    }
  }

  /// Creates a new `Trace` node with the specified log level.
  ///
  /// # Arguments
  ///
  /// * `log_level` - The logging level to use for tracing.
  pub fn with_log_level(log_level: TraceLevel) -> Self {
    Self {
      transformer: TraceTransformer::with_log_level(log_level),
    }
  }

  /// Sets the error handling strategy for this node.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }
}

impl<T> Default for Trace<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> Clone for Trace<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for Trace<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for Trace<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for Trace<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}
