use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::traits::{
  output::Output,
  producer::{Producer, ProducerConfig},
};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio::sync::mpsc;

pub struct ChannelProducer<T: std::fmt::Debug + Clone + Send + Sync> {
  rx: mpsc::Receiver<T>,
  config: ProducerConfig<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync> ChannelProducer<T> {
  pub fn new(rx: mpsc::Receiver<T>) -> Self {
    Self {
      rx,
      config: ProducerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for ChannelProducer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Producer for ChannelProducer<T> {
  fn produce(&mut self) -> Self::OutputStream {
    let rx = std::mem::replace(&mut self.rx, mpsc::channel(1).1);
    Box::pin(futures::stream::unfold(rx, |mut rx| async move {
      match rx.recv().await {
        Some(item) => Some((item, rx)),
        None => None,
      }
    }))
  }

  fn set_config_impl(&mut self, config: ProducerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy() {
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
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "channel_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
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
    let result: Vec<i32> = stream.collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_empty_channel() {
    let (_, rx) = mpsc::channel::<i32>(10);
    let mut producer = ChannelProducer::new(rx);

    let stream = producer.produce();
    let result: Vec<i32> = stream.collect().await;

    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let mut producer = ChannelProducer::new(rx)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_producer".to_string());

    let config = producer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_producer".to_string()));
  }
}
