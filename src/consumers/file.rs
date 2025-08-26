use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::traits::{
  consumer::{Consumer, ConsumerConfig},
  input::Input,
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub struct FileConsumer {
  file: Option<File>,
  path: String,
  config: ConsumerConfig<String>,
}

impl FileConsumer {
  pub fn new(path: String) -> Self {
    Self {
      file: None,
      path,
      config: ConsumerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }
}

impl Input for FileConsumer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Consumer for FileConsumer {
  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    // Create file once at the start
    if self.file.is_none() {
      if let Ok(file) = File::create(&self.path).await {
        self.file = Some(file);
      }
    }

    while let Some(value) = stream.next().await {
      if let Some(file) = &mut self.file {
        if let Err(e) = file.write_all(value.as_bytes()).await {
          eprintln!("Failed to write to file: {}", e);
        }
        // Ensure each write is flushed
        if let Err(e) = file.flush().await {
          eprintln!("Failed to flush file: {}", e);
        }
      }
    }

    // Final flush after stream is consumed
    if let Some(file) = &mut self.file {
      if let Err(e) = file.flush().await {
        eprintln!("Failed to flush file: {}", e);
      }
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> ConsumerConfig<String> {
    self.config.clone()
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    match self.config.error_strategy {
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
  use tempfile::NamedTempFile;
  use tokio::fs as tokio_fs;

  #[tokio::test]
  async fn test_file_consumer_basic() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    let mut consumer = FileConsumer::new(path.clone());

    let input = stream::iter(vec!["line1", "line2"].into_iter().map(|s| s.to_string()));
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;

    // Use async file reading to ensure all writes are complete
    let contents = tokio_fs::read_to_string(path).await.unwrap();
    assert_eq!(contents, "line1line2");
  }

  #[tokio::test]
  async fn test_file_consumer_empty_input() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    let mut consumer = FileConsumer::new(path.clone());

    let input = stream::iter(Vec::<String>::new());
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;

    // Use async file reading to ensure all writes are complete
    let contents = tokio_fs::read_to_string(path).await.unwrap();
    assert_eq!(contents, "");
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    let consumer = FileConsumer::new(path)
      .with_error_strategy(ErrorStrategy::<String>::Skip)
      .with_name("test_consumer".to_string());

    let config = consumer.get_config();
    assert_eq!(config.error_strategy, ErrorStrategy::<String>::Skip);
    assert_eq!(config.name, "test_consumer");
  }
}
