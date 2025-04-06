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
  config: ConsumerConfig,
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
      config: ConsumerConfig::default(),
      _phantom: std::marker::PhantomData,
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
  async fn consume(&mut self, input: Self::InputStream) -> Result<(), StreamError> {
    let mut stream = input;
    while let Some(item) = stream.next().await {
      match item {
        Ok(value) => {
          let mut cmd = Command::new(&self.command);
          cmd.args(&self.args);
          cmd.arg(value.to_string());

          match cmd.output().await {
            Ok(output) => {
              if !output.status.success() {
                let error = StreamError::new(
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
                );
                let strategy = self.handle_error(error.clone());
                match strategy {
                  ErrorStrategy::Stop => return Err(error),
                  ErrorStrategy::Skip => continue,
                  ErrorStrategy::Retry(n) if error.retries < n => {
                    // Retry logic would go here
                    return Err(error);
                  }
                  _ => return Err(error),
                }
              }
            }
            Err(e) => {
              let error = StreamError::new(
                Box::new(e),
                self.create_error_context(Some(Box::new(value))),
                self.component_info(),
              );
              let strategy = self.handle_error(error.clone());
              match strategy {
                ErrorStrategy::Stop => return Err(error),
                ErrorStrategy::Skip => continue,
                ErrorStrategy::Retry(n) if error.retries < n => {
                  // Retry logic would go here
                  return Err(error);
                }
                _ => return Err(error),
              }
            }
          }
        }
        Err(e) => {
          let strategy = self.handle_error(e.clone());
          match strategy {
            ErrorStrategy::Stop => return Err(e),
            ErrorStrategy::Skip => continue,
            ErrorStrategy::Retry(n) if e.retries < n => {
              // Retry logic would go here
              return Err(e);
            }
            _ => return Err(e),
          }
        }
      }
    }
    Ok(())
  }

  fn config(&self) -> &ConsumerConfig {
    &self.config
  }

  fn config_mut(&mut self) -> &mut ConsumerConfig {
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

  fn create_error_context(&self, item: Option<Box<dyn std::any::Any + Send>>) -> ErrorContext {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Consumer,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config()
        .name()
        .unwrap_or_else(|| "command_consumer".to_string()),
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

    let config = consumer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
    assert_eq!(config.name(), Some("test_consumer".to_string()));

    let error = StreamError::new(
      Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      consumer.create_error_context(None),
      consumer.component_info(),
    );

    assert_eq!(consumer.handle_error(error), ErrorStrategy::Skip);
  }
}
