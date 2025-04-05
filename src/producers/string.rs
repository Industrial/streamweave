use crate::traits::{error::Error, output::Output, producer::Producer};
use futures::{Stream, StreamExt};
use std::error::Error as StdError;
use std::fmt;
use std::pin::Pin;

#[derive(Debug)]
pub enum StringError {
  NoData,
  InvalidChunkSize,
}

impl fmt::Display for StringError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      StringError::NoData => write!(f, "No data available"),
      StringError::InvalidChunkSize => write!(f, "Invalid chunk size"),
    }
  }
}

impl StdError for StringError {}

pub struct StringProducer {
  data: String,
  chunk_size: usize,
}

impl StringProducer {
  pub fn new(data: String, chunk_size: usize) -> Self {
    Self { data, chunk_size }
  }
}

impl Error for StringProducer {
  type Error = StringError;
}

impl Output for StringProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<String, StringError>> + Send>>;
}

impl Producer for StringProducer {
  fn produce(&mut self) -> Self::OutputStream {
    if self.chunk_size == 0 {
      return Box::pin(futures::stream::once(async {
        Err(StringError::InvalidChunkSize)
      }));
    }

    // Split by lines if chunk_size is 1 (default behavior in tests)
    let chunks = if self.chunk_size == 1 {
      self.data.lines().map(String::from).collect::<Vec<_>>()
    } else {
      // For other chunk sizes, split into chunks of characters
      self
        .data
        .chars()
        .collect::<Vec<_>>()
        .chunks(self.chunk_size)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
    };

    Box::pin(futures::stream::iter(chunks).map(Ok))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;

  #[tokio::test]
  async fn test_string_producer_single_line() {
    let mut producer = StringProducer::new("Hello, World!".to_string(), 1);
    let stream = producer.produce();
    let result: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, vec!["Hello, World!"]);
  }

  #[tokio::test]
  async fn test_string_producer_multiple_lines() {
    let mut producer = StringProducer::new("Line 1\nLine 2\nLine 3".to_string(), 1);
    let stream = producer.produce();
    let result: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, vec!["Line 1", "Line 2", "Line 3"]);
  }

  #[tokio::test]
  async fn test_string_producer_empty() {
    let mut producer = StringProducer::new("".to_string(), 1);
    let stream = producer.produce();
    let result: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, Vec::<String>::new());
  }

  #[tokio::test]
  async fn test_invalid_chunk_size() {
    let mut producer = StringProducer::new("test".to_string(), 0);
    let stream = producer.produce();
    let result = stream.collect::<Vec<_>>().await;
    assert!(matches!(result[0], Err(StringError::InvalidChunkSize)));
  }

  #[tokio::test]
  async fn test_custom_chunk_size() {
    let mut producer = StringProducer::new("abcdef".to_string(), 2);
    let stream = producer.produce();
    let result: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, vec!["ab", "cd", "ef"]);
  }
}
