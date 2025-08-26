use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::traits::{
  output::Output,
  producer::{Producer, ProducerConfig},
};
use futures::Stream;
use std::pin::Pin;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

pub struct CommandProducer {
  command: String,
  args: Vec<String>,
  config: ProducerConfig<String>,
}

impl CommandProducer {
  pub fn new(command: impl Into<String>, args: Vec<impl Into<String>>) -> Self {
    Self {
      command: command.into(),
      args: args.into_iter().map(|arg| arg.into()).collect(),
      config: ProducerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Output for CommandProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Producer for CommandProducer {
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

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;

  #[tokio::test]
  async fn test_command_producer_echo() {
    let mut producer = CommandProducer::new("echo", vec!["Hello, World!"]);
    let stream = producer.produce();
    let result: Vec<String> = stream.collect().await;
    assert_eq!(result, vec!["Hello, World!"]);
  }

  #[tokio::test]
  #[cfg(unix)] // This test is Unix-specific
  async fn test_command_producer_multiple_lines() {
    let mut producer = CommandProducer::new("sh", vec!["-c", "echo 'line1\nline2\nline3'"]);
    let stream = producer.produce();
    let result: Vec<String> = stream.collect().await;
    assert_eq!(result, vec!["line1", "line2", "line3"]);
  }

  #[tokio::test]
  async fn test_command_producer_nonexistent() {
    let mut producer = CommandProducer::new("nonexistentcommand", Vec::<String>::new());
    let stream = producer.produce();
    let result: Vec<String> = stream.collect().await;
    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_multiple_calls() {
    let mut producer = CommandProducer::new("echo", vec!["test"]);

    // First call
    let stream = producer.produce();
    let result1: Vec<String> = stream.collect().await;
    assert_eq!(result1, vec!["test"]);

    // Second call
    let stream = producer.produce();
    let result2: Vec<String> = stream.collect().await;
    assert_eq!(result2, vec!["test"]);
  }

  #[tokio::test]
  #[cfg(unix)]
  async fn test_command_error_handling() {
    let mut producer = CommandProducer::new("sh", vec!["-c", "echo test && false"]);
    let stream = producer.produce();
    let result: Vec<String> = stream.collect().await;
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

    let error = StreamError {
      source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "CommandProducer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "CommandProducer".to_string(),
      },
      retries: 0,
    };

    assert_eq!(producer.handle_error(&error), ErrorAction::Skip);
  }
}
