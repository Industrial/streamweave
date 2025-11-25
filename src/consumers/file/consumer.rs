use super::file_consumer::FileConsumer;
use crate::consumer::{Consumer, ConsumerConfig};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use async_trait::async_trait;
use futures::StreamExt;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{error, warn};

#[async_trait]
impl Consumer for FileConsumer {
  /// Consumes a stream and writes each item to the file.
  ///
  /// # Error Handling
  ///
  /// - If the file cannot be created, an error is logged and all items are dropped.
  /// - If a write fails, an error is logged but consumption continues.
  /// - If a flush fails, a warning is logged but consumption continues.
  ///
  /// Note: The configured error strategy is not currently applied to I/O errors
  /// during stream consumption. This is a known limitation of the current architecture.
  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    let component_name = self.config.name.clone();
    let path = self.path.clone();

    // Create file once at the start
    if self.file.is_none() {
      match File::create(&self.path).await {
        Ok(file) => {
          self.file = Some(file);
        }
        Err(e) => {
          error!(
            component = %component_name,
            path = %path,
            error = %e,
            "Failed to create file, all items will be dropped"
          );
        }
      }
    }

    while let Some(value) = stream.next().await {
      if let Some(file) = &mut self.file {
        if let Err(e) = file.write_all(value.as_bytes()).await {
          error!(
            component = %component_name,
            path = %path,
            error = %e,
            "Failed to write to file"
          );
        }
        // Ensure each write is flushed
        if let Err(e) = file.flush().await {
          warn!(
            component = %component_name,
            path = %path,
            error = %e,
            "Failed to flush file"
          );
        }
      }
    }

    // Final flush after stream is consumed
    if let Some(file) = &mut self.file
      && let Err(e) = file.flush().await
    {
      warn!(
        component = %component_name,
        path = %path,
        error = %e,
        "Failed to perform final flush"
      );
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<String> {
    &mut self.config
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
  use crate::error::ErrorStrategy;
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

    assert_eq!(
      consumer.config().error_strategy,
      ErrorStrategy::<String>::Skip
    );
    assert_eq!(consumer.config().name, "test_consumer");
  }
}
