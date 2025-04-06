use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  error::Error,
  output::Output,
  producer::{Producer, ProducerConfig},
};
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio_stream::wrappers::LinesStream;

pub struct CommandProducer {
  command: String,
  args: Vec<String>,
  config: ProducerConfig,
}

impl CommandProducer {
  pub fn new(command: impl Into<String>, args: Vec<impl Into<String>>) -> Self {
    Self {
      command: command.into(),
      args: args.into_iter().map(|arg| arg.into()).collect(),
      config: ProducerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy) -> Self {
    self.config_mut().set_error_strategy(strategy);
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config_mut().set_name(name);
    self
  }
}

impl Error for CommandProducer {
  type Error = StreamError;
}

impl Output for CommandProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<String, StreamError>> + Send>>;
}

impl Producer for CommandProducer {
  fn produce(&mut self) -> Self::OutputStream {
    let command_str = self.command.clone();
    let args = self.args.clone();
    let config = self.config.clone();

    // Create a stream that first spawns the command and then yields its output
    let stream = async_stream::try_stream! {
        let child = Command::new(&command_str)
            .args(&args)
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| StreamError::new(
                Box::new(e),
                self.create_error_context(None),
                self.component_info(),
            ))?;

        let stdout = child.stdout
            .ok_or_else(|| StreamError::new(
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to get stdout",
                )),
                self.create_error_context(None),
                self.component_info(),
            ))?;

        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line()
            .await
            .map_err(|e| StreamError::new(
                Box::new(e),
                self.create_error_context(None),
                self.component_info(),
            ))?
        {
            yield line;
        }
    };

    Box::pin(stream)
  }

  fn config(&self) -> &ProducerConfig {
    &self.config
  }

  fn config_mut(&mut self) -> &mut ProducerConfig {
    &mut self.config
  }

  fn handle_error(&self, error: StreamError) -> ErrorStrategy {
    match self.config().error_strategy() {
      ErrorStrategy::Stop => ErrorStrategy::Stop,
      ErrorStrategy::Skip => ErrorStrategy::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorStrategy::Retry(n),
      _ => ErrorStrategy::Stop,
    }
  }

  fn create_error_context(
    &self,
    item: Option<Arc<dyn std::any::Any + Send + Sync>>,
  ) -> ErrorContext {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Producer,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config()
        .name()
        .unwrap_or_else(|| "command_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;

  #[tokio::test]
  async fn test_command_producer_echo() {
    let mut producer = CommandProducer::new("echo", vec!["Hello, World!"]);
    let stream = producer.produce();
    let result: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, vec!["Hello, World!"]);
  }

  #[tokio::test]
  #[cfg(unix)] // This test is Unix-specific
  async fn test_command_producer_multiple_lines() {
    let mut producer = CommandProducer::new("sh", vec!["-c", "echo 'line1\nline2\nline3'"]);
    let stream = producer.produce();
    let result: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, vec!["line1", "line2", "line3"]);
  }

  #[tokio::test]
  async fn test_command_producer_nonexistent() {
    let mut producer = CommandProducer::new("nonexistentcommand", Vec::<String>::new());
    let stream = producer.produce();
    let result = stream.collect::<Vec<_>>().await;
    assert!(matches!(result[0], Err(StreamError { .. })));
  }

  #[tokio::test]
  async fn test_multiple_calls() {
    let mut producer = CommandProducer::new("echo", vec!["test"]);

    // First call
    let stream = producer.produce();
    let result1: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result1, vec!["test"]);

    // Second call
    let stream = producer.produce();
    let result2: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result2, vec!["test"]);
  }

  #[tokio::test]
  #[cfg(unix)]
  async fn test_command_error_handling() {
    let mut producer = CommandProducer::new("sh", vec!["-c", "echo test && false"]);
    let stream = producer.produce();
    let result: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, vec!["test"]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut producer = CommandProducer::new("echo", vec!["test"])
      .with_error_strategy(ErrorStrategy::Skip)
      .with_name("test_producer".to_string());

    let config = producer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
    assert_eq!(config.name(), Some("test_producer".to_string()));

    let error = StreamError::new(
      Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      producer.create_error_context(None),
      producer.component_info(),
    );

    assert_eq!(producer.handle_error(error), ErrorStrategy::Skip);
  }
}
