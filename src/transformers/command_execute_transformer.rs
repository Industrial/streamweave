//! Command execute transformer for executing shell commands dynamically.
//!
//! This module provides [`CommandExecuteTransformer`], a transformer that executes
//! shell commands for each input item, running commands dynamically based on input
//! data and producing command output as a stream.
//!
//! # Overview
//!
//! [`CommandExecuteTransformer`] executes shell commands for each input item. It
//! supports flexible input formats (JSON objects with command/args or simple command
//! strings) and streams command stdout line-by-line. This is useful for integrating
//! external tools, scripts, and command-line utilities into stream processing pipelines.
//!
//! # Key Concepts
//!
//! - **Dynamic Command Execution**: Executes shell commands based on input items
//! - **Flexible Input**: Accepts JSON objects with command/args or simple command strings
//! - **Output Streaming**: Produces command stdout as a stream of lines
//! - **Async Execution**: Uses Tokio's `Command` for asynchronous process execution
//! - **Error Handling**: Configurable error strategies for command failures
//!
//! # Core Types
//!
//! - **[`CommandExecuteTransformer`]**: Transformer that executes shell commands
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust,no_run
//! use streamweave::transformers::CommandExecuteTransformer;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer that executes commands
//! let transformer = CommandExecuteTransformer::new();
//!
//! // Input: ["{\"command\": \"echo\", \"args\": [\"hello\"]}", ...]
//! // Output: ["hello", ...]
//! # Ok(())
//! # }
//! ```
//!
//! ## Input Formats
//!
//! The transformer accepts two input formats:
//!
//! - **JSON Object**: `{"command": "echo", "args": ["hello", "world"]}`
//! - **Command String**: `"echo"` (executed with no arguments)
//!
//! # Design Decisions
//!
//! - **Dynamic Execution**: Commands are executed dynamically based on input data
//! - **Async Process**: Uses Tokio's async process execution for non-blocking I/O
//! - **Line-by-Line Output**: Streams stdout line-by-line for efficient processing
//! - **Flexible Input**: Supports both structured (JSON) and simple (string) command specs
//! - **Error Handling**: Configurable error strategies for command failures
//!
//! # Integration with StreamWeave
//!
//! [`CommandExecuteTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline or graph. It supports the standard error handling
//! strategies and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::error;

/// A transformer that executes shell commands from input items.
///
/// Input can be:
/// - A JSON object with `command` and `args` fields
/// - A simple command string (executed with no arguments)
///
/// Output is a stream of lines from the command's stdout.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::CommandExecuteTransformer;
///
/// let transformer = CommandExecuteTransformer::new();
/// // Input: ["{\"command\": \"echo\", \"args\": [\"hello\"]}", ...]
/// // Output: ["hello", ...]
/// ```
pub struct CommandExecuteTransformer {
  /// Transformer configuration
  config: TransformerConfig<String>,
}

impl CommandExecuteTransformer {
  /// Creates a new `CommandExecuteTransformer`.
  #[must_use]
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this transformer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Default for CommandExecuteTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for CommandExecuteTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for CommandExecuteTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for CommandExecuteTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for CommandExecuteTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "command_execute_transformer".to_string());
    let error_strategy = self.config.error_strategy.clone();

    Box::pin(async_stream::stream! {
      let mut input_stream = input;
      while let Some(item) = input_stream.next().await {
        // Parse input - could be JSON object with command/args or simple command string
        let (command, args) = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&item) {
          // JSON object with command and args
          let cmd = json
            .get("command")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| item.clone());
          let cmd_args = json
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| {
              arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<String>>()
            })
            .unwrap_or_default();
          (cmd, cmd_args)
        } else {
          // Simple command string - execute with no arguments
          (item, Vec::new())
        };

        // Execute command
        match execute_command(&command, &args).await {
          Ok(mut output_lines) => {
            while let Some(line) = output_lines.next().await {
              yield line;
            }
          }
          Err(e) => {
            let stream_error = StreamError::new(
              e,
              ErrorContext {
                timestamp: chrono::Utc::now(),
                item: Some(format!("command: {}, args: {:?}", command, args)),
                component_name: component_name.clone(),
                component_type: std::any::type_name::<CommandExecuteTransformer>().to_string(),
              },
              ComponentInfo {
                name: component_name.clone(),
                type_name: std::any::type_name::<CommandExecuteTransformer>().to_string(),
              },
            );
            match handle_error_strategy(&error_strategy, &stream_error) {
              ErrorAction::Stop => {
                error!(
                  component = %component_name,
                  command = %command,
                  error = %stream_error,
                  "Stopping due to command execution error"
                );
                return;
              }
              ErrorAction::Skip => {
                // Continue to next command
              }
              ErrorAction::Retry => {
                // Retry not directly supported for command execution
              }
            }
          }
        }
      }
    })
  }

  fn set_config_impl(&mut self, config: TransformerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<String> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "command_execute_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "command_execute_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

async fn execute_command(
  command: &str,
  args: &[String],
) -> Result<Pin<Box<dyn Stream<Item = String> + Send>>, Box<dyn std::error::Error + Send + Sync>> {
  let child = Command::new(command)
    .args(args)
    .stdout(Stdio::piped())
    .spawn()?;

  let stdout = child
    .stdout
    .ok_or_else(|| "Failed to capture stdout".to_string())?;

  let reader = BufReader::new(stdout);
  let mut lines = reader.lines();

  let stream = async_stream::stream! {
    while let Some(line_result) = lines.next_line().await.transpose() {
      match line_result {
        Ok(line) => yield line,
        Err(e) => {
          error!("Failed to read line from command output: {}", e);
          break;
        }
      }
    }
  };

  Ok(Box::pin(stream))
}

/// Helper function to handle error strategy
pub(crate) fn handle_error_strategy<T>(
  strategy: &ErrorStrategy<T>,
  error: &StreamError<T>,
) -> ErrorAction
where
  T: std::fmt::Debug + Clone + Send + Sync,
{
  match strategy {
    ErrorStrategy::Stop => ErrorAction::Stop,
    ErrorStrategy::Skip => ErrorAction::Skip,
    ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
    ErrorStrategy::Custom(handler) => handler(error),
    _ => ErrorAction::Stop,
  }
}
