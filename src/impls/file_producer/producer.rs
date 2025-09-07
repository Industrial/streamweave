use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::structs::file_producer::FileProducer;
use crate::traits::producer::{Producer, ProducerConfig};
use async_trait::async_trait;
use futures::StreamExt;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_stream::wrappers::LinesStream;

#[async_trait]
impl Producer for FileProducer {
  fn produce(&mut self) -> Self::OutputStream {
    let path = self.path.clone();

    Box::pin(async_stream::stream! {
      if let Ok(file) = File::open(&path).await {
        let reader = BufReader::new(file);
        let mut lines = LinesStream::new(reader.lines());

        while let Some(line) = lines.next().await {
          if let Ok(line) = line {
            yield line;
          }
        }
      }
    })
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
        .unwrap_or_else(|| "file_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
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
  use std::io::Write;
  use tempfile::NamedTempFile;

  #[tokio::test]
  async fn test_file_producer() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "line 1").unwrap();
    writeln!(file, "line 2").unwrap();
    writeln!(file, "line 3").unwrap();
    file.flush().unwrap();
    let path = file.path().to_str().unwrap().to_string();
    let mut producer = FileProducer::new(path);
    let stream = producer.produce();
    let result: Vec<String> = stream.collect().await;
    assert_eq!(result, vec!["line 1", "line 2", "line 3"]);
    drop(file);
  }

  #[tokio::test]
  async fn test_empty_file() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap().to_string();
    let mut producer = FileProducer::new(path);
    let stream = producer.produce();
    let result: Vec<String> = stream.collect().await;
    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_nonexistent_file() {
    let mut producer = FileProducer::new("nonexistent.txt".to_string());
    let stream = producer.produce();
    let result: Vec<String> = stream.collect().await;
    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let producer = FileProducer::new("test.txt".to_string())
      .with_error_strategy(ErrorStrategy::Skip)
      .with_name("test_producer".to_string());

    let config = producer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
    assert_eq!(config.name(), Some("test_producer".to_string()));

    let error = StreamError {
      source: Box::new(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "test error",
      )),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "FileProducer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "FileProducer".to_string(),
      },
      retries: 0,
    };

    assert_eq!(producer.handle_error(&error), ErrorAction::Skip);
  }
}
