//! Process execute node for StreamWeave graphs
//!
//! Executes external processes from stream items. Takes process parameters as input
//! and outputs process results, enabling dynamic process execution in a pipeline.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::ProcessExecuteTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that executes external processes from stream items.
///
/// This node wraps `ProcessExecuteTransformer` for use in graphs. It takes process
/// parameters (as JSON or simple strings) as input and outputs process results,
/// enabling dynamic process execution in a pipeline.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{ProcessExecute, TransformerNode};
///
/// let process_execute = ProcessExecute::new();
/// let node = TransformerNode::from_transformer(
///     "process_execute".to_string(),
///     process_execute,
/// );
/// ```
pub struct ProcessExecute {
  /// The underlying process execute transformer
  transformer: ProcessExecuteTransformer,
}

impl ProcessExecute {
  /// Creates a new `ProcessExecute` node.
  pub fn new() -> Self {
    Self {
      transformer: ProcessExecuteTransformer::new(),
    }
  }

  /// Sets the error handling strategy for this node.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
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

impl Default for ProcessExecute {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for ProcessExecute {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for ProcessExecute {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for ProcessExecute {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for ProcessExecute {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<String>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<String> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<String> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}
