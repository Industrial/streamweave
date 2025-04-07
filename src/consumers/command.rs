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
  command: Option<Command>,
  config: ConsumerConfig<T>,
}

impl<T> CommandConsumer<T>
where
  T: Send + Sync + 'static + std::fmt::Debug + std::fmt::Display,
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
  T: Send + Sync + 'static + std::fmt::Debug + std::fmt::Display,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Consumer for CommandConsumer<T>
where
  T: Send + Sync + 'static + std::fmt::Debug + std::fmt::Display,
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

  fn get_config_impl(&self) -> &ConsumerConfig<T> {
    &self.config
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
      stage: PipelineStage::Consumer(self.component_info().name),
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
    let mut consumer = CommandConsumer::new("echo".to_string(), vec![])
      .with_error_strategy(ErrorStrategy::<&str>::Skip)
      .with_name("test_consumer".to_string());

    let config = consumer.get_config();
    assert_eq!(config.error_strategy, ErrorStrategy::<&str>::Skip);
    assert_eq!(config.name, "test_consumer");
  }
}
