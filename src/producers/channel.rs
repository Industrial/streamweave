use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  error::Error,
  output::Output,
  producer::{Producer, ProducerConfig},
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::error::Error as StdError;
use std::fmt;
use std::pin::Pin;
use tokio::sync::mpsc;

pub struct ChannelProducer<T> {
  rx: mpsc::Receiver<T>,
  config: ProducerConfig,
}

impl<T> ChannelProducer<T> {
  pub fn new(rx: mpsc::Receiver<T>) -> Self {
    Self {
      rx,
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

impl<T: Send + 'static> Error for ChannelProducer<T> {
  type Error = StreamError;
}

impl<T: Send + 'static> Output for ChannelProducer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<T, StreamError>> + Send>>;
}

#[async_trait]
impl<T: Send + 'static> Producer for ChannelProducer<T> {
  fn produce(&mut self) -> Self::OutputStream {
    let mut rx = self.rx.clone();
    let config = self.config.clone();

    Box::pin(futures::stream::unfold(
      (rx, config),
      |(mut rx, config)| async move {
        match rx.recv().await {
          Some(item) => Some((Ok(item), (rx, config))),
          None => None,
        }
      },
    ))
  }

  fn config(&self) -> &ProducerConfig {
    &self.config
  }

  fn config_mut(&mut self) -> &mut ProducerConfig {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError) -> ErrorAction {
    match self.config().error_strategy() {
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
      stage: PipelineStage::Producer,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config()
        .name()
        .unwrap_or_else(|| "channel_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;
  use tokio::sync::mpsc;

  #[tokio::test]
  async fn test_channel_producer() {
    let (tx, rx) = mpsc::channel(10);
    let mut producer = ChannelProducer::new(rx);

    tx.send(1).await.unwrap();
    tx.send(2).await.unwrap();
    tx.send(3).await.unwrap();
    drop(tx);

    let stream = producer.produce();
    let result: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_empty_channel() {
    let (_, rx) = mpsc::channel::<i32>(10);
    let mut producer = ChannelProducer::new(rx);

    let stream = producer.produce();
    let result: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;

    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let (_, rx) = mpsc::channel::<i32>(10);
    let mut producer = ChannelProducer::new(rx)
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

    assert_eq!(producer.handle_error(&error), ErrorAction::Skip);
  }
}
