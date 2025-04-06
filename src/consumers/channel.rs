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
  config: ConsumerConfig,
  sender: Sender<T>,
}

impl<T> ChannelConsumer<T>
where
  T: Send + 'static,
{
  pub fn new(sender: Sender<T>) -> Self {
    Self {
      config: ConsumerConfig::default(),
      sender,
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
        .unwrap_or_else(|| "channel_consumer".to_string()),
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
