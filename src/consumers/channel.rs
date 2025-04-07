use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tokio::sync::mpsc::{Receiver, Sender, channel};

use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  consumer::{Consumer, ConsumerConfig},
  input::Input,
};

pub struct ChannelConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  channel: Option<Sender<T>>,
  config: ConsumerConfig<T>,
}

impl<T> ChannelConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new(sender: Sender<T>) -> Self {
    Self {
      channel: Some(sender),
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

impl<T> Input for ChannelConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Consumer for ChannelConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  async fn consume(&mut self, input: Self::InputStream) -> () {
    let mut stream = input;
    while let Some(value) = stream.next().await {
      if let Some(sender) = &self.channel {
        if let Err(e) = sender.send(value).await {
          eprintln!("Failed to send value to channel: {}", e);
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
  async fn test_channel_consumer_basic() {
    let (tx, mut rx) = channel(10);
    let mut consumer = ChannelConsumer::new(tx);

    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;

    assert_eq!(rx.recv().await, Some(1));
    assert_eq!(rx.recv().await, Some(2));
    assert_eq!(rx.recv().await, Some(3));
    assert_eq!(rx.recv().await, None);
  }

  #[tokio::test]
  async fn test_channel_consumer_empty_input() {
    let (tx, mut rx) = channel(10);
    let mut consumer = ChannelConsumer::new(tx);

    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    assert_eq!(rx.recv().await, None);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let (tx, _rx) = channel(10);
    let mut consumer = ChannelConsumer::new(tx)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_consumer".to_string());

    let config = consumer.get_config();
    assert_eq!(config.error_strategy, ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name, "test_consumer");
  }

  #[tokio::test]
  async fn test_error_handling_during_consumption() {
    let (tx, _rx) = channel(10);
    let mut consumer = ChannelConsumer::new(tx)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_consumer".to_string());

    // Test that Skip strategy allows consumption to continue
    let action = consumer.handle_error(&StreamError {
      source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: Some(42),
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
    let mut consumer = consumer.with_error_strategy(ErrorStrategy::<i32>::Stop);
    let action = consumer.handle_error(&StreamError {
      source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: Some(42),
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
    let (tx, _rx): (Sender<i32>, Receiver<i32>) = channel(10);
    let consumer = ChannelConsumer::new(tx).with_name("test_consumer".to_string());

    let info = consumer.component_info();
    assert_eq!(info.name, "test_consumer");
    assert_eq!(
      info.type_name,
      "streamweave::consumers::channel::ChannelConsumer<i32>"
    );
  }

  #[tokio::test]
  async fn test_error_context_creation() {
    let (tx, _rx): (Sender<i32>, Receiver<i32>) = channel(10);
    let consumer = ChannelConsumer::new(tx).with_name("test_consumer".to_string());

    let context = consumer.create_error_context(Some(42));
    assert_eq!(context.component_name, "test_consumer");
    assert_eq!(
      context.component_type,
      "streamweave::consumers::channel::ChannelConsumer<i32>"
    );
    assert_eq!(context.item, Some(42));
  }

  #[tokio::test]
  async fn test_channel_capacity() {
    let (tx, mut rx): (Sender<i32>, Receiver<i32>) = channel(2); // Small capacity
    let mut consumer = ChannelConsumer::new(tx);

    let input = stream::iter(vec![1, 2, 3, 4]);
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;

    // Should receive all items despite capacity limit
    assert_eq!(rx.recv().await, Some(1));
    assert_eq!(rx.recv().await, Some(2));
    assert_eq!(rx.recv().await, Some(3));
    assert_eq!(rx.recv().await, Some(4));
    assert_eq!(rx.recv().await, None);
  }

  #[tokio::test]
  async fn test_dropped_channel() {
    let (tx, rx): (Sender<i32>, Receiver<i32>) = channel(10);
    let mut consumer = ChannelConsumer::new(tx);

    // Drop the receiver
    drop(rx);

    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    // Should not panic when sending to dropped channel
    consumer.consume(boxed_input).await;
  }
}
