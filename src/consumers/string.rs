use crate::traits::{consumer::Consumer, error::Error, input::Input};
use async_trait::async_trait;
use futures::StreamExt;
use std::pin::Pin;

#[derive(Debug)]
pub struct StringConsumer {
  result: String,
  separator: Option<String>,
}

#[derive(Debug)]
pub enum StringConsumerError {
  JoinError(String),
}

impl std::fmt::Display for StringConsumerError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      StringConsumerError::JoinError(e) => write!(f, "Failed to join strings: {}", e),
    }
  }
}

impl std::error::Error for StringConsumerError {}

impl Error for StringConsumer {
  type Error = StringConsumerError;
}

impl Input for StringConsumer {
  type Input = String;
  type InputStream = Pin<Box<dyn futures::Stream<Item = Result<Self::Input, Self::Error>> + Send>>;
}

impl StringConsumer {
  pub fn new() -> Self {
    Self {
      result: String::new(),
      separator: None,
    }
  }

  pub fn with_separator(separator: impl Into<String>) -> Self {
    Self {
      result: String::new(),
      separator: Some(separator.into()),
    }
  }

  pub fn into_string(self) -> String {
    self.result
  }

  pub fn as_str(&self) -> &str {
    &self.result
  }
}

impl Default for StringConsumer {
  fn default() -> Self {
    Self::new()
  }
}

#[async_trait]
impl Consumer for StringConsumer {
  async fn consume(&mut self, mut input: Self::InputStream) -> Result<(), Self::Error> {
    let mut is_first = self.result.is_empty(); // Only true if no content exists
    while let Some(result) = input.next().await {
      let value = result?;
      if !is_first {
        if let Some(sep) = &self.separator {
          self.result.push_str(sep);
        }
      }
      self.result.push_str(&value);
      is_first = false;
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;
  use tokio;

  #[tokio::test]
  async fn test_empty_stream() {
    let mut consumer = StringConsumer::new();
    let input = stream::empty();
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert_eq!(consumer.as_str(), "");
    assert_eq!(consumer.into_string(), "");
  }

  #[tokio::test]
  async fn test_single_string() {
    let mut consumer = StringConsumer::new();
    let input = stream::iter(vec![Ok("hello".to_string())]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert_eq!(consumer.as_str(), "hello");
    assert_eq!(consumer.into_string(), "hello");
  }

  #[tokio::test]
  async fn test_multiple_strings_no_separator() {
    let mut consumer = StringConsumer::new();
    let input = stream::iter(vec![
      Ok("hello".to_string()),
      Ok("world".to_string()),
      Ok("!".to_string()),
    ]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert_eq!(consumer.as_str(), "helloworld!");
  }

  #[tokio::test]
  async fn test_multiple_strings_with_separator() {
    let mut consumer = StringConsumer::with_separator(", ");
    let input = stream::iter(vec![
      Ok("hello".to_string()),
      Ok("world".to_string()),
      Ok("!".to_string()),
    ]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert_eq!(consumer.as_str(), "hello, world, !");
  }

  #[tokio::test]
  async fn test_empty_strings() {
    let mut consumer = StringConsumer::with_separator("|");
    let input = stream::iter(vec![
      Ok("".to_string()),
      Ok("hello".to_string()),
      Ok("".to_string()),
      Ok("world".to_string()),
      Ok("".to_string()),
    ]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert_eq!(consumer.as_str(), "|hello||world|");
  }

  #[tokio::test]
  async fn test_error_propagation() {
    let mut consumer = StringConsumer::new();
    let input = stream::iter(vec![
      Ok("hello".to_string()),
      Err(StringConsumerError::JoinError("test error".to_string())),
      Ok("world".to_string()),
    ]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_err());
    assert!(matches!(
      result.unwrap_err(),
      StringConsumerError::JoinError(_)
    ));
    assert_eq!(consumer.as_str(), "hello");
  }

  #[tokio::test]
  async fn test_default_impl() {
    let consumer = StringConsumer::default();
    assert_eq!(consumer.as_str(), "");
  }

  #[tokio::test]
  async fn test_large_strings() {
    let mut consumer = StringConsumer::with_separator("\n");
    let strings: Vec<_> = (0..1000).map(|i| Ok(format!("line_{}", i))).collect();
    let input = stream::iter(strings);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());

    let result = consumer.into_string();
    let lines: Vec<_> = result.split('\n').collect();
    assert_eq!(lines.len(), 1000);
    for i in 0..1000 {
      assert_eq!(lines[i], format!("line_{}", i));
    }
  }

  #[tokio::test]
  async fn test_unicode_strings() {
    let mut consumer = StringConsumer::with_separator(" ");
    let input = stream::iter(vec![
      Ok("こんにちは".to_string()),
      Ok("世界".to_string()),
      Ok("!".to_string()),
    ]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert_eq!(consumer.as_str(), "こんにちは 世界 !");
  }

  #[tokio::test]
  async fn test_special_characters() {
    let mut consumer = StringConsumer::with_separator("\t");
    let input = stream::iter(vec![
      Ok("hello\nworld".to_string()),
      Ok("foo\r\nbar".to_string()),
      Ok("baz\tqux".to_string()),
    ]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert_eq!(consumer.as_str(), "hello\nworld\tfoo\r\nbar\tbaz\tqux");
  }

  #[tokio::test]
  async fn test_repeated_consumption() {
    let mut consumer = StringConsumer::with_separator("|");
    let input1 = stream::iter(vec![Ok("hello".to_string())]);
    let result1 = consumer.consume(Box::pin(input1)).await;
    assert!(result1.is_ok());
    assert_eq!(consumer.as_str(), "hello");

    let input2 = stream::iter(vec![Ok("world".to_string())]);
    let result2 = consumer.consume(Box::pin(input2)).await;
    assert!(result2.is_ok());
    assert_eq!(consumer.as_str(), "hello|world");
  }
}
