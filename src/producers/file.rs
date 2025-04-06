use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  error::Error,
  output::Output,
  producer::{Producer, ProducerConfig},
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::error::Error as StdError;
use std::fmt;
use std::io::{self, BufRead};
use std::pin::Pin;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

pub struct FileProducer {
  path: String,
  config: ProducerConfig,
}

impl FileProducer {
  pub fn new(path: String) -> Self {
    Self {
      path,
      config: ProducerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy) -> Self {
    self.config_mut().set_error_strategy(strategy);
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config_mut().set_name(name);
    self
  }
}

impl Error for FileProducer {
  type Error = StreamError;
}

impl Output for FileProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<String, StreamError>> + Send>>;
}

#[async_trait]
impl Producer for FileProducer {
  fn produce(&mut self) -> Self::OutputStream {
    let path = self.path.clone();
    let config = self.config.clone();

    Box::pin(futures::stream::unfold(
      (path, config),
      |(path, config)| async move {
        match File::open(&path).await {
          Ok(file) => {
            let mut reader = BufReader::new(file);
            let mut line = String::new();
            match reader.read_line(&mut line).await {
              Ok(0) => None,
              Ok(_) => Some((Ok(line.trim().to_string()), (path, config))),
              Err(e) => Some((
                Err(StreamError::new(
                  Box::new(e),
                  ErrorContext {
                    timestamp: chrono::Utc::now(),
                    item: None,
                    stage: PipelineStage::Producer,
                  },
                  ComponentInfo {
                    name: config.name().unwrap_or_else(|| "file_producer".to_string()),
                    type_name: std::any::type_name::<Self>().to_string(),
                  },
                )),
                (path, config),
              )),
            }
          }
          Err(e) => Some((
            Err(StreamError::new(
              Box::new(e),
              ErrorContext {
                timestamp: chrono::Utc::now(),
                item: None,
                stage: PipelineStage::Producer,
              },
              ComponentInfo {
                name: config.name().unwrap_or_else(|| "file_producer".to_string()),
                type_name: std::any::type_name::<Self>().to_string(),
              },
            )),
            (path, config),
          )),
        }
      },
    ))
  }

  fn config(&self) -> &ProducerConfig {
    &self.config
  }

  fn config_mut(&mut self) -> &mut ProducerConfig {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError) -> ErrorAction {
    match self.config().error_strategy() {
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
      stage: PipelineStage::Producer,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config()
        .name()
        .unwrap_or_else(|| "file_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;
  use tempfile::NamedTempFile;
  use tokio::io::AsyncWriteExt;

  #[tokio::test]
  async fn test_file_producer() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "line 1").unwrap();
    writeln!(file, "line 2").unwrap();
    writeln!(file, "line 3").unwrap();
    let path = file.path().to_str().unwrap().to_string();

    let mut producer = FileProducer::new(path);
    let stream = producer.produce();
    let result: Vec<String> = stream.map(|r| r.unwrap()).collect().await;

    assert_eq!(result, vec!["line 1", "line 2", "line 3"]);
  }

  #[tokio::test]
  async fn test_empty_file() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap().to_string();

    let mut producer = FileProducer::new(path);
    let stream = producer.produce();
    let result: Vec<String> = stream.map(|r| r.unwrap()).collect().await;

    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_nonexistent_file() {
    let mut producer = FileProducer::new("nonexistent.txt".to_string());
    let stream = producer.produce();
    let result = stream.collect::<Vec<_>>().await;
    assert!(result[0].is_err());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut producer = FileProducer::new("test.txt".to_string())
      .with_error_strategy(ErrorStrategy::Skip)
      .with_name("test_producer".to_string());

    let config = producer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
    assert_eq!(config.name(), Some("test_producer".to_string()));

    let error = StreamError::new(
      Box::new(io::Error::new(io::ErrorKind::NotFound, "test error")),
      producer.create_error_context(None),
      producer.component_info(),
    );

    assert_eq!(producer.handle_error(&error), ErrorAction::Skip);
  }
}
