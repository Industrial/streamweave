//! Process transformer for StreamWeave
//!
//! Executes external processes on stream items. Each item is passed to a process
//! as input, and the process output is returned as the transformed item.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{error, warn};

/// A transformer that executes external processes on stream items.
///
/// Each input item is passed to an external process (as stdin or command argument),
/// and the process output (stdout) is returned as the transformed item.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::ProcessTransformer;
///
/// // Execute a command for each item
/// let transformer = ProcessTransformer::new("echo".to_string(), vec![]);
/// // Input: ["hello", "world"]
/// // Output: ["hello\n", "world\n"]
/// ```
pub struct ProcessTransformer {
  /// Command to execute
  command: String,
  /// Command arguments
  args: Vec<String>,
  /// Whether to pass input as stdin (true) or as command argument (false)
  use_stdin: bool,
  /// Configuration for the transformer
  config: TransformerConfig<String>,
}

impl ProcessTransformer {
  /// Creates a new `ProcessTransformer` that executes the specified command.
  ///
  /// Input items will be passed as command-line arguments by default.
  ///
  /// # Arguments
  ///
  /// * `command` - The command to execute.
  /// * `args` - Additional command-line arguments (before the input item).
  pub fn new(command: String, args: Vec<String>) -> Self {
    Self {
      command,
      args,
      use_stdin: false,
      config: TransformerConfig::default(),
    }
  }

  /// Creates a new `ProcessTransformer` that passes input via stdin.
  ///
  /// # Arguments
  ///
  /// * `command` - The command to execute.
  /// * `args` - Command-line arguments.
  pub fn with_stdin(command: String, args: Vec<String>) -> Self {
    Self {
      command,
      args,
      use_stdin: true,
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
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

impl Clone for ProcessTransformer {
  fn clone(&self) -> Self {
    Self {
      command: self.command.clone(),
      args: self.args.clone(),
      use_stdin: self.use_stdin,
      config: self.config.clone(),
    }
  }
}

impl Input for ProcessTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for ProcessTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for ProcessTransformer {
  type InputPorts = (String,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let command = self.command.clone();
    let args = self.args.clone();
    let use_stdin = self.use_stdin;

    Box::pin(
      input
        .then(move |item| {
          let command = command.clone();
          let args = args.clone();
          async move {
            let mut cmd = Command::new(&command);
            cmd.args(&args);
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            if use_stdin {
              cmd.stdin(Stdio::piped());
            } else {
              // Pass item as command argument
              cmd.arg(&item);
            }

            match cmd.spawn() {
              Ok(mut child) => {
                if use_stdin && let Some(mut stdin) = child.stdin.take() {
                  use tokio::io::AsyncWriteExt;
                  if let Err(e) = stdin.write_all(item.as_bytes()).await {
                    error!("Failed to write to process stdin: {}", e);
                    return None;
                  }
                  drop(stdin);
                }

                match child.wait_with_output().await {
                  Ok(output) => {
                    if output.status.success() {
                      String::from_utf8(output.stdout)
                        .ok()
                        .map(|s| s.trim_end().to_string())
                    } else {
                      warn!(
                        command = %command,
                        status = ?output.status,
                        stderr = %String::from_utf8_lossy(&output.stderr),
                        "Process execution failed"
                      );
                      None
                    }
                  }
                  Err(e) => {
                    error!(command = %command, error = %e, "Failed to wait for process");
                    None
                  }
                }
              }
              Err(e) => {
                error!(command = %command, error = %e, "Failed to spawn process");
                None
              }
            }
          }
        })
        .filter_map(futures::future::ready),
    )
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
    match self.config.error_strategy {
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
        .unwrap_or_else(|| "process_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
