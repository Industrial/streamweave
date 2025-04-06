use crate::traits::{error::Error, output::Output, producer::Producer};
use futures::{Stream, StreamExt};
use std::error::Error as StdError;
use std::fmt;
use std::pin::Pin;
use tokio::sync::mpsc::{self, Sender};
use tokio_stream::wrappers::ReceiverStream;

#[derive(Debug)]
pub enum ChannelError {
  SendError(String),
}

impl fmt::Display for ChannelError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ChannelError::SendError(msg) => write!(f, "Channel send error: {}", msg),
    }
  }
}

impl StdError for ChannelError {}

pub struct ChannelProducer<T> {
  receiver: Option<mpsc::Receiver<T>>,
  buffer_size: usize,
}

impl<T> ChannelProducer<T> {
  pub fn new(buffer_size: usize) -> (Self, Sender<T>) {
    let (tx, rx) = mpsc::channel(buffer_size);
    (
      Self {
        receiver: Some(rx),
        buffer_size,
      },
      tx,
    )
  }

  pub fn with_default_buffer() -> (Self, Sender<T>) {
    Self::new(32)
  }

  pub fn buffer_size(&self) -> usize {
    self.buffer_size
  }
}

impl<T: Send + 'static> Error for ChannelProducer<T> {
  type Error = ChannelError;
}

impl<T: Send + 'static> Output for ChannelProducer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<T, ChannelError>> + Send>>;
}

impl<T: Send + 'static> Producer for ChannelProducer<T> {
  fn produce(&mut self) -> Self::OutputStream {
    let receiver = self.receiver.take().expect("Stream already taken");
    let stream = ReceiverStream::new(receiver).map(Ok);
    Box::pin(stream)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;
  use tokio::sync::mpsc::error::TrySendError;

  #[tokio::test]
  async fn test_channel_producer() {
    let (mut producer, sender) = ChannelProducer::new(2);

    // Start consuming in a separate task
    let stream = producer.produce();
    let collect_task =
      tokio::spawn(async move { stream.map(|r| r.unwrap()).collect::<Vec<i32>>().await });

    // Send data
    sender.send(1).await.unwrap();
    sender.send(2).await.unwrap();
    sender.send(3).await.unwrap();
    drop(sender);

    // Wait for collection to complete
    let result = collect_task.await.unwrap();
    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_channel_producer_empty() {
    let (mut producer, sender) = ChannelProducer::<i32>::new(2);
    drop(sender);

    let stream = producer.produce();
    let result: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_channel_buffer_size() {
    let (producer, sender) = ChannelProducer::<i32>::new(2);
    assert_eq!(producer.buffer_size(), 2);

    // Test that buffer size is enforced
    sender.try_send(1).unwrap();
    sender.try_send(2).unwrap();
    assert!(matches!(sender.try_send(3), Err(TrySendError::Full(3))));
  }

  #[tokio::test]
  async fn test_default_buffer() {
    let (producer, _) = ChannelProducer::<i32>::with_default_buffer();
    assert_eq!(producer.buffer_size(), 32);
  }

  #[tokio::test]
  #[should_panic(expected = "Stream already taken")]
  async fn test_double_produce() {
    let (mut producer, _) = ChannelProducer::<i32>::new(2);
    let _stream1 = producer.produce();
    let _stream2 = producer.produce(); // Should panic
  }

  #[tokio::test]
  async fn test_multiple_senders() {
    let (mut producer, sender1) = ChannelProducer::new(4);
    let sender2 = sender1.clone();

    // Start consuming in a separate task
    let stream = producer.produce();
    let collect_task =
      tokio::spawn(async move { stream.map(|r| r.unwrap()).collect::<Vec<i32>>().await });

    // Spawn two tasks that send data
    let task1 = tokio::spawn(async move {
      sender1.send(1).await.unwrap();
      sender1.send(3).await.unwrap();
    });

    let task2 = tokio::spawn(async move {
      sender2.send(2).await.unwrap();
      sender2.send(4).await.unwrap();
    });

    // Wait for sends to complete
    task1.await.unwrap();
    task2.await.unwrap();

    // Wait for collection and check results
    let mut result = collect_task.await.unwrap();
    result.sort(); // Order may vary due to concurrent sends
    assert_eq!(result, vec![1, 2, 3, 4]);
  }
}
