use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::error::Error as StdError;
use std::fmt;
use std::pin::Pin;
use tokio::sync::mpsc::{Receiver, Sender, channel};

use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  consumer::{Consumer, ConsumerConfig},
  input::Input,
};

#[derive(Debug)]
pub enum ChannelError {
  SendError(String),
  Consumption(ConsumptionError),
}

impl fmt::Display for ChannelError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ChannelError::SendError(msg) => write!(f, "Channel send error: {}", msg),
      ChannelError::Consumption(e) => write!(f, "{}", e),
    }
  }
}

impl StdError for ChannelError {
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
    match self {
      ChannelError::SendError(_) => None,
      ChannelError::Consumption(e) => Some(e),
    }
  }
}

impl ConsumptionErrorInspection for ChannelError {
  fn is_consumption_error(&self) -> bool {
    matches!(self, ChannelError::Consumption(_))
  }

  fn as_consumption_error(&self) -> Option<&ConsumptionError> {
    match self {
      ChannelError::Consumption(e) => Some(e),
      _ => None,
    }
  }
}

pub struct ChannelConsumer<T> {
  sender: Sender<T>,
}

impl<T> ChannelConsumer<T>
where
  T: Send + 'static,
{
  pub fn new(sender: Sender<T>) -> Self {
    Self { sender }
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

impl<T> crate::traits::error::Error for ChannelConsumer<T>
where
  T: Send + 'static,
{
  type Error = StreamError;
}

impl<T> Input for ChannelConsumer<T>
where
  T: Send + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError>> + Send>>;
}

#[async_trait]
impl<T> Consumer for ChannelConsumer<T>
where
  T: Send + 'static,
{
  async fn consume(&mut self, input: Self::InputStream) -> Result<(), StreamError> {
    let mut stream = input;
    while let Some(item) = stream.next().await {
      match item {
        Ok(value) => {
          self.sender.send(value).await.map_err(|e| {
            StreamError::new(
              Box::new(e),
              self.create_error_context(None),
              self.component_info(),
            )
          })?;
        }
        Err(e) => {
          let action = self.handle_error(&e);
          match action {
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

  fn create_error_context(&self, item: Option<Box<dyn std::any::Any + Send>>) -> ErrorContext {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Consumer,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    let config = self.get_config();
    ComponentInfo {
      name: config.name,
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;

  #[tokio::test]
  async fn test_channel_consumer_basic() {
    let (tx, mut rx) = channel(10);
    let mut consumer = ChannelConsumer::new(tx);

    let input = stream::iter(vec![1, 2, 3].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_ok());

    assert_eq!(rx.recv().await, Some(1));
    assert_eq!(rx.recv().await, Some(2));
    assert_eq!(rx.recv().await, Some(3));
    assert_eq!(rx.recv().await, None);
  }

  #[tokio::test]
  async fn test_channel_consumer_empty_input() {
    let (tx, mut rx) = channel(10);
    let mut consumer = ChannelConsumer::new(tx);

    let input = stream::iter(Vec::<Result<i32, StreamError>>::new());
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_ok());
    assert_eq!(rx.recv().await, None);
  }

  #[tokio::test]
  async fn test_channel_consumer_with_error() {
    let (tx, mut rx) = channel(10);
    let mut consumer = ChannelConsumer::new(tx);

    let input = stream::iter(vec![
      Ok(1),
      Err(StreamError::new(
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
        ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          stage: PipelineStage::Consumer,
        },
        ComponentInfo {
          name: "test".to_string(),
          type_name: "test".to_string(),
        },
      )),
      Ok(2),
    ]);
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_err());
    assert_eq!(rx.recv().await, Some(1));
    assert_eq!(rx.recv().await, None);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let (tx, _rx) = channel(10);
    let mut consumer = ChannelConsumer::new(tx)
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
