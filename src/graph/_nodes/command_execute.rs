//! Command execute node for executing shell commands in graphs.
//!
//! This module provides [`CommandExecute`], a graph node that executes shell commands
//! from stream items. It takes command templates or parameters as input and outputs
//! command results, enabling dynamic command execution in graph-based pipelines.
//! It wraps [`CommandExecuteTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`CommandExecute`] is useful for integrating external command-line tools and scripts
//! into StreamWeave graphs. It allows each stream item to trigger a command execution,
//! making it ideal for dynamic command invocation based on stream data.
//!
//! # Key Concepts
//!
//! - **Command Execution**: Executes shell commands for each stream item
//! - **Dynamic Commands**: Supports command templates and parameter substitution
//! - **Result Output**: Outputs command execution results (stdout, stderr, exit codes)
//! - **Transformer Wrapper**: Wraps `CommandExecuteTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`CommandExecute`]**: Node that executes shell commands from stream items
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::CommandExecute;
//!
//! // Create a command execute node
//! let command_execute = CommandExecute::new();
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::CommandExecute;
//! use streamweave::ErrorStrategy;
//!
//! // Create a command execute node with error handling
//! let command_execute = CommandExecute::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("command-executor".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Shell Command Integration**: Enables integration with external command-line
//!   tools and scripts
//! - **Dynamic Execution**: Supports command templates and parameter substitution
//!   for flexible command execution
//! - **Result Capture**: Captures command output, errors, and exit codes
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`CommandExecute`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::CommandExecuteTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that executes shell commands from stream items.
///
/// This node wraps `CommandExecuteTransformer` for use in graphs. It takes command
/// templates/parameters (as JSON or simple strings) as input and outputs command results,
/// enabling dynamic command execution in a pipeline.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{CommandExecute, TransformerNode};
///
/// let command_execute = CommandExecute::new();
/// let node = TransformerNode::from_transformer(
///     "command_execute".to_string(),
///     command_execute,
/// );
/// ```
pub struct CommandExecute {
  /// The underlying command execute transformer
  transformer: CommandExecuteTransformer,
}

impl CommandExecute {
  /// Creates a new `CommandExecute` node.
  pub fn new() -> Self {
    Self {
      transformer: CommandExecuteTransformer::new(),
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

impl Default for CommandExecute {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for CommandExecute {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for CommandExecute {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for CommandExecute {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for CommandExecute {
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
