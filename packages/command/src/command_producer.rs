use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use std::process::Stdio;
use streamweave::{Output, Producer, ProducerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// A producer that executes shell commands and produces their output.
///
/// This producer runs a command with the specified arguments and emits
/// each line of output as a separate item.
pub struct CommandProducer {
  /// The command to execute.
  pub command: String,
  /// The arguments to pass to the command.
  pub args: Vec<String>,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<String>,
}

impl CommandProducer {
  /// Creates a new `CommandProducer` with the given command and arguments.
  ///
  /// # Arguments
  ///
  /// * `command` - The command to execute.
  /// * `args` - The arguments to pass to the command.
  pub fn new(command: impl Into<String>, args: Vec<impl Into<String>>) -> Self {
    Self {
      command: command.into(),
      args: args.into_iter().map(|arg| arg.into()).collect(),
      config: streamweave::ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this producer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this producer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

// Trait implementations for CommandProducer

impl Output for CommandProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Producer for CommandProducer {
  type OutputPorts = (String,);

  fn produce(&mut self) -> Self::OutputStream {
    let command_str = self.command.clone();
    let args = self.args.clone();
    let _config = self.config.clone();

    // Create a stream that first spawns the command and then yields its output
    let stream = async_stream::stream! {
        let child = match Command::new(&command_str)
            .args(&args)
            .stdout(Stdio::piped())
            .spawn() {
            Ok(child) => child,
            Err(_) => return,
        };

        let stdout = match child.stdout {
            Some(stdout) => stdout,
            None => return,
        };

        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await.ok().flatten() {
            yield line;
        }
    };

    Box::pin(stream)
  }

  fn set_config_impl(&mut self, config: ProducerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<String> {
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
        .unwrap_or_else(|| "command_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "command_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
