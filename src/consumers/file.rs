use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
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
}

impl FileConsumer {
  pub fn new(path: String) -> Self {
    Self { file: None, path }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy) -> Self {
    let mut config = self.get_config();
    config.error_strategy = strategy;
    self.set_config(config);
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    let mut config = self.get_config();
    config.name = name;
    self.set_config(config);
    self
  }
}

impl crate::traits::error::Error for FileConsumer {
  type Error = StreamError;
}

impl Input for FileConsumer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError>> + Send>>;
}

#[async_trait]
impl Consumer for FileConsumer {
  async fn consume(&mut self, input: Self::InputStream) -> Result<(), StreamError> {
    let mut stream = input;
    while let Some(item) = stream.next().await {
      match item {
        Ok(value) => {
          if self.file.is_none() {
            self.file = Some(File::create(&self.path).await.map_err(|e| {
              StreamError::new(
                Box::new(e),
                self.create_error_context(None),
                self.component_info(),
              )
            })?);
          }

          if let Some(file) = &mut self.file {
            file.write_all(value.as_bytes()).await.map_err(|e| {
              StreamError::new(
                Box::new(e),
                self.create_error_context(Some(Box::new(value))),
                self.component_info(),
              )
            })?;
          }
        }
        Err(e) => {
          let action = self.handle_error(&e);
          match action {
            ErrorAction::Stop => return Err(e),
            ErrorAction::Skip => continue,
            ErrorAction::Retry => {
              // Retry logic would go here
              return Err(e);
            }
          }
        }
      }
    }
    Ok(())
  }

  fn handle_error(&self, error: &StreamError) -> ErrorAction {
    match self.get_config().error_strategy {
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
      stage: PipelineStage::Consumer,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    let config = self.get_config();
    ComponentInfo {
      name: config.name,
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;
  use std::fs;
  use tempfile::NamedTempFile;

  #[tokio::test]
  async fn test_file_consumer_basic() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    let mut consumer = FileConsumer::new(path.clone());

    let input = stream::iter(
      vec!["line1", "line2", "line3"]
        .into_iter()
        .map(|s| Ok(s.to_string())),
    );
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_ok());

    let contents = fs::read_to_string(path).unwrap();
    assert_eq!(contents, "line1\nline2\nline3\n");
  }

  #[tokio::test]
  async fn test_file_consumer_empty_input() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    let mut consumer = FileConsumer::new(path.clone());

    let input = stream::iter(Vec::<Result<String, StreamError>>::new());
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_ok());

    let contents = fs::read_to_string(path).unwrap();
    assert_eq!(contents, "");
  }

  #[tokio::test]
  async fn test_file_consumer_with_error() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    let mut consumer = FileConsumer::new(path);

    let input = stream::iter(vec![
      Ok("line1".to_string()),
      Err(StreamError::new(
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
        ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          stage: PipelineStage::Consumer,
        },
        ComponentInfo {
          name: "test".to_string(),
          type_name: "test".to_string(),
        },
      )),
      Ok("line2".to_string()),
    ]);
    let boxed_input = Box::pin(input);

    let result = consumer.consume(boxed_input).await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    let mut consumer = FileConsumer::new(path)
      .with_error_strategy(ErrorStrategy::Skip)
      .with_name("test_consumer".to_string());

    let config = consumer.get_config();
    assert_eq!(config.error_strategy, ErrorStrategy::Skip);
    assert_eq!(config.name, "test_consumer");

    let error = StreamError::new(
      Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      consumer.create_error_context(None),
      consumer.component_info(),
    );

    assert_eq!(consumer.handle_error(&error), ErrorAction::Skip);
  }
}
