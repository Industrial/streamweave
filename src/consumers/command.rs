use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  consumer::{Consumer, ConsumerConfig},
  input::Input,
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tokio::process::Command;

pub struct CommandConsumer<T> {
  command: String,
  args: Vec<String>,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> CommandConsumer<T>
where
  T: Send + 'static,
{
  pub fn new(command: String, args: Vec<String>) -> Self {
    Self {
      command,
      args,
      _phantom: std::marker::PhantomData,
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy) -> Self {
    let mut config = self.get_config();
    config.error_strategy = strategy;
    self.set_config(config);
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    let mut config = self.get_config();
    config.name = name;
    self.set_config(config);
    self
  }
}

impl<T> crate::traits::error::Error for CommandConsumer<T>
where
  T: Send + 'static,
{
  type Error = StreamError;
}

impl<T> Input for CommandConsumer<T>
where
  T: Send + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError>> + Send>>;
}

#[async_trait]
impl<T> Consumer for CommandConsumer<T>
where
  T: Send + 'static + std::fmt::Display,
{
  async fn consume(&mut self, mut stream: Self::InputStream) -> Result<(), StreamError> {
    while let Some(item) = stream.next().await {
      match item {
        Ok(value) => {
          let command = format!("{}", value);
          let output = Command::new(&self.command)
            .arg(&command)
            .output()
            .await
            .map_err(|e| {
              StreamError::new(
                Box::new(e),
                self.create_error_context(Some(Box::new(value))),
                self.component_info(),
              )
            })?;

          if !output.status.success() {
            return Err(StreamError::new(
              Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                  "Command failed with status {}: {}",
                  output.status,
                  String::from_utf8_lossy(&output.stderr)
                ),
              )),
              self.create_error_context(Some(Box::new(value))),
              self.component_info(),
            ));
          }
        }
        Err(e) => {
          let strategy = self.handle_error(&e);
          match strategy {
            ErrorAction::Stop => return Err(e),
            ErrorAction::Skip => continue,
            ErrorAction::Retry => {
              // Retry logic would go here
              return Err(e);
            }
          }
        }
      }
    }
    Ok(())
  }

  fn handle_error(&self, error: &StreamError) -> ErrorAction {
    match self.get_config().error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    let config = self.get_config();
    ComponentInfo {
      name: config.name,
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }

  fn create_error_context(&self, item: Option<Box<dyn std::any::Any + Send>>) -> ErrorContext {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Consumer,
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
    let input = stream::iter(vec!["test"].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_command_consumer_empty_input() {
    let mut consumer = CommandConsumer::new("echo".to_string(), vec![]);
    let input = stream::iter(Vec::<Result<&str, StreamError>>::new());
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_command_consumer_with_error() {
    let mut consumer = CommandConsumer::new("nonexistent_command".to_string(), vec![]);
    let input = stream::iter(vec![Ok("test")]);
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut consumer = CommandConsumer::new("echo".to_string(), vec![])
      .with_error_strategy(ErrorStrategy::Skip)
      .with_name("test_consumer".to_string());

    let config = consumer.get_config();
    assert_eq!(config.error_strategy, ErrorStrategy::Skip);
    assert_eq!(config.name, "test_consumer");

    let error = StreamError::new(
      Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      consumer.create_error_context(None),
      consumer.component_info(),
    );

    assert_eq!(consumer.handle_error(&error), ErrorAction::Skip);
  }
}
