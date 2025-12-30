use futures::Stream;
use std::pin::Pin;
use std::process::Stdio;
use streamweave::{Output, Producer, ProducerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// A producer that reads output from an external process.
///
/// Spawns a process and emits its stdout line by line.
pub struct ProcessProducer {
  /// The command to execute
  pub command: String,
  /// Command arguments
  pub args: Vec<String>,
  /// Configuration for the producer
  pub config: ProducerConfig<String>,
}

impl ProcessProducer {
  /// Creates a new `ProcessProducer` with the given command.
  ///
  /// # Arguments
  ///
  /// * `command` - The command to execute
  pub fn new(command: String) -> Self {
    Self {
      command,
      args: Vec::new(),
      config: ProducerConfig::default(),
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

  /// Sets the name for this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Output for ProcessProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Producer for ProcessProducer {
  type OutputPorts = (String,);

  fn produce(&mut self) -> Self::OutputStream {
    let command = self.command.clone();
    let args = self.args.clone();

    Box::pin(async_stream::stream! {
      let child = match Command::new(&command)
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
    })
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
        .unwrap_or_else(|| "process_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "process_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
