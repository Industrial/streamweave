use super::file_consumer::FileConsumer;
use async_trait::async_trait;
use futures::StreamExt;
use streamweave_core::{Consumer, ConsumerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{error, warn};

#[async_trait]
impl Consumer for FileConsumer {
  type InputPorts = (String,);

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
  use futures::stream;
  use proptest::prelude::*;
  use proptest::proptest;
  use streamweave_error::ErrorStrategy;
  use tempfile::NamedTempFile;
  use tokio::fs as tokio_fs;
  use tokio::runtime::Runtime;

  async fn test_file_consumer_basic_async(input_data: Vec<String>) {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    let mut consumer = FileConsumer::new(path.clone());

    let expected_contents = input_data.concat();
    let input = stream::iter(input_data.clone());
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;

    // Use async file reading to ensure all writes are complete
    let contents = tokio_fs::read_to_string(path).await.unwrap();
    assert_eq!(contents, expected_contents);
    drop(temp_file);
  }

  proptest! {
    #[test]
    fn test_file_consumer_basic(input_data in prop::collection::vec(any::<String>(), 0..20)) {
      let rt = Runtime::new().unwrap();
      rt.block_on(test_file_consumer_basic_async(input_data));
    }
  }

  async fn test_file_consumer_empty_input_async() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    let mut consumer = FileConsumer::new(path.clone());

    let input = stream::iter(Vec::<String>::new());
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;

    // Use async file reading to ensure all writes are complete
    let contents = tokio_fs::read_to_string(path).await.unwrap();
    assert_eq!(contents, "");
    drop(temp_file);
  }

  proptest! {
    #[test]
    fn test_file_consumer_empty_input(_ in prop::num::u8::ANY) {
      let rt = Runtime::new().unwrap();
      rt.block_on(test_file_consumer_empty_input_async());
    }
  }

  proptest! {
    #[test]
    fn test_error_handling_strategies(
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
    ) {
      let temp_file = NamedTempFile::new().unwrap();
      let path = temp_file.path().to_str().unwrap().to_string();
      let consumer = FileConsumer::new(path)
        .with_error_strategy(ErrorStrategy::<String>::Skip)
        .with_name(name.clone());

      prop_assert_eq!(
        &consumer.config().error_strategy,
        &ErrorStrategy::<String>::Skip
      );
      prop_assert_eq!(consumer.config().name.as_str(), name.as_str());
    }
  }

  #[tokio::test]
  async fn test_set_config_impl() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    let mut consumer = FileConsumer::new(path);
    let new_config = ConsumerConfig::<String> {
      name: "test_consumer".to_string(),
      error_strategy: ErrorStrategy::<String>::Retry(5),
    };

    consumer.set_config_impl(new_config.clone());
    assert_eq!(consumer.get_config_impl().name, new_config.name);
    assert_eq!(
      consumer.get_config_impl().error_strategy,
      new_config.error_strategy
    );
  }

  #[tokio::test]
  async fn test_get_config_mut_impl() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    let mut consumer = FileConsumer::new(path);
    let config_mut = consumer.get_config_mut_impl();
    config_mut.name = "mutated_name".to_string();
    assert_eq!(consumer.get_config_impl().name, "mutated_name");
  }

  #[tokio::test]
  async fn test_handle_error_stop() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    let consumer = FileConsumer::new(path).with_error_strategy(ErrorStrategy::<String>::Stop);
    let error = StreamError::new(
      Box::new(std::io::Error::other("test error")),
      ErrorContext::default(),
      ComponentInfo::default(),
    );
    assert_eq!(consumer.handle_error(&error), ErrorAction::Stop);
  }

  #[tokio::test]
  async fn test_handle_error_skip() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    let consumer = FileConsumer::new(path).with_error_strategy(ErrorStrategy::<String>::Skip);
    let error = StreamError::new(
      Box::new(std::io::Error::other("test error")),
      ErrorContext::default(),
      ComponentInfo::default(),
    );
    assert_eq!(consumer.handle_error(&error), ErrorAction::Skip);
  }

  #[tokio::test]
  async fn test_handle_error_retry_within_limit() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    let consumer = FileConsumer::new(path).with_error_strategy(ErrorStrategy::<String>::Retry(5));
    let mut error = StreamError::new(
      Box::new(std::io::Error::other("test error")),
      ErrorContext::default(),
      ComponentInfo::default(),
    );
    error.retries = 3;
    assert_eq!(consumer.handle_error(&error), ErrorAction::Retry);
  }

  #[tokio::test]
  async fn test_handle_error_retry_exceeds_limit() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    let consumer = FileConsumer::new(path).with_error_strategy(ErrorStrategy::<String>::Retry(5));
    let mut error = StreamError::new(
      Box::new(std::io::Error::other("test error")),
      ErrorContext::default(),
      ComponentInfo::default(),
    );
    error.retries = 5;
    assert_eq!(consumer.handle_error(&error), ErrorAction::Stop);
  }

  #[tokio::test]
  async fn test_create_error_context() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    let consumer = FileConsumer::new(path).with_name("test_consumer".to_string());
    let context = consumer.create_error_context(Some("test_item".to_string()));
    assert_eq!(context.item, Some("test_item".to_string()));
    assert_eq!(context.component_name, "test_consumer");
    assert!(context.timestamp <= chrono::Utc::now());
  }

  #[tokio::test]
  async fn test_create_error_context_no_item() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    let consumer = FileConsumer::new(path).with_name("test_consumer".to_string());
    let context = consumer.create_error_context(None);
    assert_eq!(context.item, None);
    assert_eq!(context.component_name, "test_consumer");
  }

  #[tokio::test]
  async fn test_component_info() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    let consumer = FileConsumer::new(path).with_name("test_consumer".to_string());
    let info = consumer.component_info();
    assert_eq!(info.name, "test_consumer");
    assert_eq!(info.type_name, std::any::type_name::<FileConsumer>());
  }

  #[tokio::test]
  async fn test_component_info_default_name() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    let consumer = FileConsumer::new(path);
    let info = consumer.component_info();
    assert_eq!(info.name, "");
    assert_eq!(info.type_name, std::any::type_name::<FileConsumer>());
  }

  #[tokio::test]
  async fn test_consume_with_existing_file() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap().to_string();
    let mut consumer = FileConsumer::new(path.clone());

    // Create file first
    let _file = File::create(&path).await.unwrap();
    consumer.file = Some(_file);

    let input = stream::iter(vec!["test1".to_string(), "test2".to_string()]);
    let boxed_input = Box::pin(input);
    consumer.consume(boxed_input).await;

    let contents = tokio_fs::read_to_string(path).await.unwrap();
    assert_eq!(contents, "test1test2");
  }
}
