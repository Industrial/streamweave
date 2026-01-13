//! Process node for StreamWeave graphs
//!
//! Executes external processes on stream items. Each item is passed to a process
//! as input, and the process output is returned as the transformed item.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::ProcessTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that executes external processes on stream items.
///
/// This node wraps `ProcessTransformer` for use in graphs. Each input item is passed
/// to an external process (as stdin or command argument), and the process output
/// (stdout) is returned as the transformed item.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{Process, TransformerNode};
///
/// // Execute a command for each item
/// let process = Process::new("echo".to_string(), vec![]);
/// let node = TransformerNode::from_transformer(
///     "process".to_string(),
///     process,
/// );
/// ```
pub struct Process {
  /// The underlying process transformer
  transformer: ProcessTransformer,
}

impl Process {
  /// Creates a new `Process` node that executes the specified command.
  ///
  /// Input items will be passed as command-line arguments by default.
  ///
  /// # Arguments
  ///
  /// * `command` - The command to execute.
  /// * `args` - Additional command-line arguments (before the input item).
  pub fn new(command: String, args: Vec<String>) -> Self {
    Self {
      transformer: ProcessTransformer::new(command, args),
    }
  }

  /// Creates a new `Process` node that passes input via stdin.
  ///
  /// # Arguments
  ///
  /// * `command` - The command to execute.
  /// * `args` - Command-line arguments.
  pub fn with_stdin(command: String, args: Vec<String>) -> Self {
    Self {
      transformer: ProcessTransformer::with_stdin(command, args),
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

impl Clone for Process {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for Process {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for Process {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for Process {
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
