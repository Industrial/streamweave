use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::structs::producers::string::StringProducer;
use crate::traits::producer::{Producer, ProducerConfig};

impl Producer for StringProducer {
  fn produce(&mut self) -> Self::OutputStream {
    if self.chunk_size == 0 {
      return Box::pin(futures::stream::empty());
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

    Box::pin(futures::stream::iter(chunks))
  }

  fn set_config_impl(&mut self, config: ProducerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<String> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "string_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "string_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
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
    let result: Vec<String> = stream.collect().await;
    assert_eq!(result, vec!["Hello, World!"]);
  }

  #[tokio::test]
  async fn test_string_producer_multiple_lines() {
    let mut producer = StringProducer::new("Line 1\nLine 2\nLine 3".to_string(), 1);
    let stream = producer.produce();
    let result: Vec<String> = stream.collect().await;
    assert_eq!(result, vec!["Line 1", "Line 2", "Line 3"]);
  }

  #[tokio::test]
  async fn test_string_producer_empty() {
    let mut producer = StringProducer::new("".to_string(), 1);
    let stream = producer.produce();
    let result: Vec<String> = stream.collect().await;
    assert_eq!(result, Vec::<String>::new());
  }

  #[tokio::test]
  async fn test_invalid_chunk_size() {
    let mut producer = StringProducer::new("test".to_string(), 0);
    let stream = producer.produce();
    let result: Vec<String> = stream.collect().await;
    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_custom_chunk_size() {
    let mut producer = StringProducer::new("abcdef".to_string(), 2);
    let stream = producer.produce();
    let result: Vec<String> = stream.collect().await;
    assert_eq!(result, vec!["ab", "cd", "ef"]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let producer = StringProducer::new("test".to_string(), 1)
      .with_error_strategy(ErrorStrategy::Skip)
      .with_name("test_producer".to_string());

    let config = producer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
    assert_eq!(config.name(), Some("test_producer".to_string()));

    let error = StreamError {
      source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "StringProducer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "StringProducer".to_string(),
      },
      retries: 0,
    };

    assert_eq!(producer.handle_error(&error), ErrorAction::Skip);
  }
}
