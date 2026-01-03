use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::process::Stdio;
use streamweave::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave::{Consumer, ConsumerConfig, Input};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::warn;

/// A consumer that sends items to an external process's stdin.
pub struct ProcessConsumer {
  /// The command to execute
  pub command: String,
  /// Command arguments
  pub args: Vec<String>,
  /// Configuration for the consumer
  pub config: ConsumerConfig<String>,
}

impl ProcessConsumer {
  /// Creates a new `ProcessConsumer` with the given command.
  ///
  /// # Arguments
  ///
  /// * `command` - The command to execute
  pub fn new(command: String) -> Self {
    Self {
      command,
      args: Vec::new(),
      config: ConsumerConfig::default(),
    }
  }

  /// Adds an argument to the command.
  pub fn arg(mut self, arg: String) -> Self {
    self.args.push(arg);
    self
  }

  /// Sets the error handling strategy.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this consumer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }
}

impl Input for ProcessConsumer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Consumer for ProcessConsumer {
  type InputPorts = (String,);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    let command = self.command.clone();
    let args = self.args.clone();
    let component_name = self.config.name.clone();

    let mut child = match Command::new(&command)
      .args(&args)
      .stdin(Stdio::piped())
      .stdout(Stdio::null())
      .stderr(Stdio::null())
      .spawn()
    {
      Ok(child) => child,
      Err(e) => {
        warn!(
          component = %component_name,
          command = %command,
          error = %e,
          "Failed to spawn process, all items will be dropped"
        );
        return;
      }
    };

    if let Some(mut stdin) = child.stdin.take() {
      while let Some(value) = stream.next().await {
        if let Err(e) = stdin.write_all(value.as_bytes()).await {
          warn!(
            component = %component_name,
            command = %command,
            error = %e,
            "Failed to write to process stdin"
          );
          break;
        }
        if let Err(e) = stdin.write_all(b"\n").await {
          warn!(
            component = %component_name,
            command = %command,
            error = %e,
            "Failed to write newline to process stdin"
          );
          break;
        }
      }
      if let Err(e) = stdin.flush().await {
        warn!(
          component = %component_name,
          command = %command,
          error = %e,
          "Failed to flush process stdin"
        );
      }
    }

    // Wait for process to complete
    let _ = child.wait().await;
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<String> {
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
      component_name: self.config.name.clone(),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self.config.name.clone(),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
