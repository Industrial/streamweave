use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::traits::{
  consumer::{Consumer, ConsumerConfig},
  input::Input,
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tokio::process::Command;

pub struct CommandConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  command: Option<Command>,
  config: ConsumerConfig<T>,
}

impl<T> CommandConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  pub fn new(command: String, args: Vec<String>) -> Self {
    let mut cmd = Command::new(command);
    cmd.args(args);
    Self {
      command: Some(cmd),
      config: ConsumerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }
}

impl<T> Input for CommandConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Consumer for CommandConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    while let Some(value) = stream.next().await {
      if let Some(cmd) = &mut self.command {
        let output = cmd.arg(value.to_string()).output().await;
        if let Err(e) = output {
          eprintln!("Failed to execute command: {}", e);
        }
      }
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> ConsumerConfig<T> {
    self.config.clone()
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
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

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;

  #[tokio::test]
  async fn test_command_consumer_basic() {
    let mut consumer = CommandConsumer::new("echo".to_string(), vec![]);
    let input = stream::iter(vec!["hello", "world"]);
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
  }

  #[tokio::test]
  async fn test_command_consumer_empty_input() {
    let mut consumer = CommandConsumer::new("echo".to_string(), vec![]);
    let input = stream::iter(Vec::<&str>::new());
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let consumer = CommandConsumer::new("echo".to_string(), vec![])
      .with_error_strategy(ErrorStrategy::<&str>::Skip)
      .with_name("test_consumer".to_string());

    let config = consumer.get_config();
    assert_eq!(config.error_strategy, ErrorStrategy::<&str>::Skip);
    assert_eq!(config.name, "test_consumer");
  }

  #[tokio::test]
  async fn test_error_handling_during_consumption() {
    let mut consumer = CommandConsumer::new("echo".to_string(), vec![])
      .with_error_strategy(ErrorStrategy::<&str>::Skip)
      .with_name("test_consumer".to_string());

    // Test that Skip strategy allows consumption to continue
    let action = consumer.handle_error(&StreamError {
      source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: Some("test"),
        component_name: "test".to_string(),
        component_type: "test".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "test".to_string(),
      },
      retries: 0,
    });
    assert_eq!(action, ErrorAction::Skip);

    // Test that Stop strategy halts consumption
    let consumer = consumer.with_error_strategy(ErrorStrategy::<&str>::Stop);
    let action = consumer.handle_error(&StreamError {
      source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: Some("test"),
        component_name: "test".to_string(),
        component_type: "test".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "test".to_string(),
      },
      retries: 0,
    });
    assert_eq!(action, ErrorAction::Stop);
  }

  #[tokio::test]
  async fn test_component_info() {
    let consumer: CommandConsumer<String> = CommandConsumer::new("echo".to_string(), vec![]);
    let info = consumer.component_info();
    assert_eq!(info.name, "");
    assert_eq!(
      info.type_name,
      "streamweave::consumers::command::CommandConsumer<alloc::string::String>"
    );
  }

  #[tokio::test]
  async fn test_error_context_creation() {
    let consumer: CommandConsumer<String> = CommandConsumer::new("echo".to_string(), vec![]);
    let context = consumer.create_error_context(Some("test".to_string()));
    assert_eq!(context.component_name, "");
    assert_eq!(
      context.component_type,
      "streamweave::consumers::command::CommandConsumer<alloc::string::String>"
    );
    assert_eq!(context.item, Some("test".to_string()));
  }

  #[tokio::test]
  async fn test_command_with_arguments() {
    let mut consumer: CommandConsumer<String> =
      CommandConsumer::new("echo".to_string(), vec!["-n".to_string()]);
    let input = stream::iter(vec!["hello".to_string(), "world".to_string()]);
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
  }

  #[tokio::test]
  async fn test_command_execution_failure() {
    let mut consumer: CommandConsumer<String> =
      CommandConsumer::new("nonexistent_command".to_string(), vec![]);
    let input = stream::iter(vec!["test".to_string()]);
    let boxed_input = Box::pin(input);

    // Should not panic when command execution fails
    consumer.consume(boxed_input).await;
  }

  #[tokio::test]
  async fn test_command_output_handling() {
    let mut consumer: CommandConsumer<String> = CommandConsumer::new("echo".to_string(), vec![]);
    let input = stream::iter(vec!["test output".to_string()]);
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
  }
}
