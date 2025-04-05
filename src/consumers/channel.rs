use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::error::Error as StdError;
use std::fmt;
use std::pin::Pin;
use tokio::sync::mpsc::{Receiver, Sender, channel};

use crate::error::{ConsumptionError, ConsumptionErrorInspection};
use crate::traits::{consumer::Consumer, error::Error, input::Input};

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
  tx: Option<Sender<T>>,
  buffer_size: usize,
}

impl<T> ChannelConsumer<T> {
  pub fn new(buffer_size: usize) -> (Self, Receiver<T>) {
    let (tx, rx) = channel(buffer_size);
    (
      Self {
        tx: Some(tx),
        buffer_size,
      },
      rx,
    )
  }
}

impl<T: Send + 'static> Error for ChannelConsumer<T> {
  type Error = ChannelError;
}

impl<T: Send + 'static> Input for ChannelConsumer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, Self::Error>> + Send>>;
}

#[async_trait]
impl<T: Send + 'static> Consumer for ChannelConsumer<T> {
  async fn consume(&mut self, mut stream: Self::InputStream) -> Result<(), Self::Error> {
    let tx = self
      .tx
      .take()
      .ok_or_else(|| ChannelError::Consumption(ConsumptionError::AlreadyConsumed))?;

    while let Some(item) = stream.next().await {
      tx.send(item?)
        .await
        .map_err(|_| ChannelError::SendError("Channel closed".to_string()))?;
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;
  use tokio::sync::mpsc::Receiver;

  async fn collect_receiver<T>(mut rx: Receiver<T>) -> Vec<T> {
    let mut result = Vec::new();
    while let Some(item) = rx.recv().await {
      result.push(item);
    }
    result
  }

  #[tokio::test]
  async fn test_channel_consumer() {
    let (mut consumer, rx) = ChannelConsumer::new(10);
    let input = vec![1, 2, 3];
    let stream = Box::pin(stream::iter(input.clone()).map(Ok));

    let handle = tokio::spawn(async move {
      consumer.consume(stream).await.unwrap();
      drop(consumer); // Close sender
    });

    let result = collect_receiver(rx).await;
    handle.await.unwrap();

    assert_eq!(result, input);
  }

  #[tokio::test]
  async fn test_channel_consumer_buffer_size() {
    let (mut consumer, rx) = ChannelConsumer::new(1);
    let input = vec![1, 2, 3, 4, 5];
    let stream = Box::pin(stream::iter(input.clone()).map(Ok));

    let handle = tokio::spawn(async move {
      consumer.consume(stream).await.unwrap();
    });

    let result = collect_receiver(rx).await;
    handle.await.unwrap();

    assert_eq!(result, input);
  }

  #[tokio::test]
  async fn test_channel_consumer_reuse_fails() {
    let (mut consumer, rx) = ChannelConsumer::new(1);
    let stream1 = Box::pin(stream::iter(vec![1]).map(Ok));
    let stream2 = Box::pin(stream::iter(vec![2]).map(Ok));

    assert!(consumer.consume(stream1).await.is_ok());

    let err = consumer.consume(stream2).await.unwrap_err();
    assert!(matches!(
      err,
      ChannelError::Consumption(ConsumptionError::AlreadyConsumed)
    ));

    drop(rx); // Clean up receiver
  }

  #[tokio::test]
  async fn test_channel_consumer_closed_receiver() {
    let (mut consumer, rx) = ChannelConsumer::new(1);
    let stream = Box::pin(stream::iter(vec![1, 2, 3]).map(Ok));

    // Drop the receiver before consuming
    drop(rx);

    let err = consumer.consume(stream).await.unwrap_err();
    assert!(matches!(err, ChannelError::SendError(_)));
  }
}
