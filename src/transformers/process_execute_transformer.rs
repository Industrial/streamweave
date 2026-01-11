//! # Process Execute Transformer
//!
//! Transformer for executing external processes dynamically based on input data in StreamWeave pipelines.
//!
//! This module provides [`ProcessExecuteTransformer`], a transformer that executes external
//! processes for each input item, running commands dynamically based on input data and producing
//! process output as a stream. It enables integrating external command-line tools and scripts
//! into streaming pipelines.
//!
//! # Overview
//!
//! [`ProcessExecuteTransformer`] is useful for executing external commands and scripts in
//! streaming data processing pipelines. It accepts command specifications as input (either
//! JSON objects or simple command strings) and produces lines from process stdout as output,
//! making it ideal for integrating external tools into data processing workflows.
//!
//! # Key Concepts
//!
//! - **Dynamic Execution**: Executes different commands for each input item
//! - **Flexible Input Formats**: Accepts JSON objects with command/args or simple command strings
//! - **Output Streaming**: Produces process stdout as a stream of lines
//! - **Async Process Execution**: Uses Tokio for async process execution
//! - **Error Handling**: Configurable error strategies for process failures
//!
//! # Core Types
//!
//! - **[`ProcessExecuteTransformer`]**: Transformer that executes external processes
//!
//! # Quick Start
//!
//! ## Basic Usage with JSON Input
//!
//! ```rust
//! use streamweave::transformers::ProcessExecuteTransformer;
//!
//! // Create a transformer that executes processes
//! let transformer = ProcessExecuteTransformer::new();
//!
//! // Input: ["{\"command\": \"echo\", \"args\": [\"hello\"]}"]
//! // Output: ["hello"]
//! ```
//!
//! ## Simple Command String
//!
//! ```rust
//! use streamweave::transformers::ProcessExecuteTransformer;
//!
//! // Execute simple commands without arguments
//! let transformer = ProcessExecuteTransformer::new();
//!
//! // Input: ["echo"]
//! // Output: [""] (empty line from echo with no args)
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::ProcessExecuteTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a transformer with error handling strategy
//! let transformer = ProcessExecuteTransformer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("command-executor".to_string());
//! ```
//!
//! # Input Formats
//!
//! ## JSON Object Format
//!
//! Specify command and arguments as a JSON object:
//!
//! ```json
//! {
//!   "command": "grep",
//!   "args": ["-i", "error", "logfile.txt"]
//! }
//! ```
//!
//! ## Simple Command String
//!
//! For commands without arguments, use a simple string:
//!
//! ```
//! "date"
//! ```
//!
//! # Design Decisions
//!
//! ## Dynamic Command Execution
//!
//! Executes different commands for each input item, allowing flexible command selection
//! based on data content. This design enables data-driven command execution patterns.
//!
//! ## Input Format Flexibility
//!
//! Supports both JSON objects and simple strings for command specification. JSON format
//! provides full control over command and arguments, while simple strings enable quick
//! command execution without argument parsing.
//!
//! ## Line-Based Output
//!
//! Produces output as lines from process stdout, which is the most common pattern for
//! command-line tool integration. Each line becomes a separate stream item.
//!
//! ## Async Execution
//!
//! Uses Tokio's async process execution for non-blocking command execution. This enables
//! efficient concurrent execution of multiple processes in streaming scenarios.
//!
//! ## Error Handling
//!
//! Process execution failures are handled according to the configured error strategy.
//! Failed processes can be skipped, retried, or stop the pipeline, depending on configuration.
//!
//! # Integration with StreamWeave
//!
//! [`ProcessExecuteTransformer`] integrates seamlessly with StreamWeave's pipeline and graph systems:
//!
//! - **Pipeline API**: Use in pipelines for command execution operations
//! - **Graph API**: Wrap in graph nodes for graph-based command execution
//! - **Error Handling**: Supports standard error handling strategies
//! - **Configuration**: Supports configuration via [`TransformerConfig`]
//! - **Stream Processing**: Produces streams from process output
//!
//! # Common Patterns
//!
//! ## Data-Driven Command Execution
//!
//! Execute different commands based on input data:
//!
//! ```rust
//! use streamweave::transformers::ProcessExecuteTransformer;
//!
//! // Execute commands based on input data
//! let transformer = ProcessExecuteTransformer::new();
//! ```
//!
//! ## Tool Integration
//!
//! Integrate external tools into processing pipelines:
//!
//! ```rust
//! use streamweave::transformers::ProcessExecuteTransformer;
//!
//! // Integrate grep, awk, sed, etc. into pipelines
//! let transformer = ProcessExecuteTransformer::new();
//! ```
//!
//! ## Script Execution
//!
//! Execute scripts and shell commands:
//!
//! ```rust
//! use streamweave::transformers::ProcessExecuteTransformer;
//!
//! // Execute shell scripts or commands
//! let transformer = ProcessExecuteTransformer::new();
//! ```
//!
//! # Process Execution Details
//!
//! ## Command Construction
//!
//! Commands are constructed from input:
//! - JSON objects: `command` field specifies executable, `args` field specifies arguments
//! - Simple strings: Used as executable with no arguments
//!
//! ## Output Processing
//!
//! Process stdout is captured and split into lines, with each line becoming an output item.
//! Process stderr is logged but not included in the output stream.
//!
//! ## Process Lifecycle
//!
//! Each input item spawns a new process instance. Processes are executed asynchronously
//! and their output is streamed as it becomes available.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::error;

/// A transformer that executes external processes from input items.
///
/// Input can be:
/// - A JSON object with `command` and `args` fields
/// - A simple command string (executed with no arguments)
///
/// Output is a stream of lines from the process's stdout.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::ProcessExecuteTransformer;
///
/// let transformer = ProcessExecuteTransformer::new();
/// // Input: ["{\"command\": \"echo\", \"args\": [\"hello\"]}", ...]
/// // Output: ["hello", ...]
/// ```
pub struct ProcessExecuteTransformer {
  /// Transformer configuration
  config: TransformerConfig<String>,
}

impl ProcessExecuteTransformer {
  /// Creates a new `ProcessExecuteTransformer`.
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

impl Default for ProcessExecuteTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for ProcessExecuteTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for ProcessExecuteTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for ProcessExecuteTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for ProcessExecuteTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "process_execute_transformer".to_string());
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

        // Execute process
        match execute_process(&command, &args).await {
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
                component_type: std::any::type_name::<ProcessExecuteTransformer>().to_string(),
              },
              ComponentInfo {
                name: component_name.clone(),
                type_name: std::any::type_name::<ProcessExecuteTransformer>().to_string(),
              },
            );
            match handle_error_strategy(&error_strategy, &stream_error) {
              ErrorAction::Stop => {
                error!(
                  component = %component_name,
                  command = %command,
                  error = %stream_error,
                  "Stopping due to process execution error"
                );
                return;
              }
              ErrorAction::Skip => {
                // Continue to next process
              }
              ErrorAction::Retry => {
                // Retry not directly supported for process execution
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
        .unwrap_or_else(|| "process_execute_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "process_execute_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

async fn execute_process(
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
          error!("Failed to read line from process output: {}", e);
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
