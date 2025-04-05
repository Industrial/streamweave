use crate::traits::{error::Error, output::Output, producer::Producer};
use futures::{Stream, StreamExt};
use std::error::Error as StdError;
use std::fmt;
use std::pin::Pin;

#[derive(Debug)]
pub enum VecProducerError {
  StreamError(String),
}

impl fmt::Display for VecProducerError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      VecProducerError::StreamError(msg) => write!(f, "Stream error: {}", msg),
    }
  }
}

impl StdError for VecProducerError {}

pub struct VecProducer<T> {
  data: Vec<T>,
}

impl<T: Clone + Send + 'static> VecProducer<T> {
  pub fn new(data: Vec<T>) -> Self {
    Self { data }
  }
}

impl<T: Clone + Send + 'static> Error for VecProducer<T> {
  type Error = VecProducerError;
}

impl<T: Clone + Send + 'static> Output for VecProducer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<T, VecProducerError>> + Send>>;
}

impl<T: Clone + Send + 'static> Producer for VecProducer<T> {
  fn produce(&mut self) -> Self::OutputStream {
    let stream = futures::stream::iter(self.data.clone().into_iter().map(Ok));
    Box::pin(stream)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;

  #[tokio::test]
  async fn test_vec_producer() {
    let data = vec!["test1".to_string(), "test2".to_string()];
    let mut producer = VecProducer::new(data.clone());
    let stream = producer.produce();
    let result: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, data);
  }

  #[tokio::test]
  async fn test_vec_producer_empty() {
    let mut producer = VecProducer::<String>::new(vec![]);
    let stream = producer.produce();
    let result: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, Vec::<String>::new());
  }

  #[tokio::test]
  async fn test_vec_producer_multiple_calls() {
    let data = vec!["test1".to_string(), "test2".to_string()];
    let mut producer = VecProducer::new(data.clone());

    // First call
    let stream = producer.produce();
    let result1: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result1, data);

    // Second call
    let stream = producer.produce();
    let result2: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result2, data);
  }

  #[tokio::test]
  async fn test_vec_producer_clone_independence() {
    let mut producer1 = VecProducer::new(vec![1, 2, 3]);
    let mut producer2 = VecProducer::new(vec![4, 5, 6]);

    let stream1 = producer1.produce();
    let stream2 = producer2.produce();

    let result1: Vec<i32> = stream1.map(|r| r.unwrap()).collect().await;
    let result2: Vec<i32> = stream2.map(|r| r.unwrap()).collect().await;

    assert_eq!(result1, vec![1, 2, 3]);
    assert_eq!(result2, vec![4, 5, 6]);
  }
}
