//! Tests for FileProducer

use futures::StreamExt;
use std::io::Write;
use streamweave::Producer;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_file::FileProducer;
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

#[tokio::test]
async fn test_file_producer_with_name() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "line 1").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();
  let producer = FileProducer::new(path).with_name("my_producer".to_string());
  assert_eq!(producer.config().name(), Some("my_producer".to_string()));
  drop(file);
}

#[tokio::test]
async fn test_file_producer_set_config_impl() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "line 1").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();
  let mut producer = FileProducer::new(path);
  let new_config = streamweave::ProducerConfig::<String> {
    name: Some("new_name".to_string()),
    error_strategy: ErrorStrategy::<String>::Retry(3),
  };
  producer.set_config_impl(new_config.clone());
  assert_eq!(producer.get_config_impl().name(), new_config.name());
  assert_eq!(
    producer.get_config_impl().error_strategy(),
    new_config.error_strategy()
  );
  drop(file);
}

#[tokio::test]
async fn test_file_producer_get_config_mut_impl() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "line 1").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();
  let mut producer = FileProducer::new(path);
  let config_mut = producer.get_config_mut_impl();
  config_mut.name = Some("mutated_name".to_string());
  assert_eq!(
    producer.get_config_impl().name(),
    Some("mutated_name".to_string())
  );
  drop(file);
}

#[tokio::test]
async fn test_file_producer_handle_error_stop() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "line 1").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();
  let producer = FileProducer::new(path).with_error_strategy(ErrorStrategy::<String>::Stop);
  let error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::default(),
  );
  assert_eq!(producer.handle_error(&error), ErrorAction::Stop);
  drop(file);
}

#[tokio::test]
async fn test_file_producer_handle_error_retry_within_limit() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "line 1").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();
  let producer = FileProducer::new(path).with_error_strategy(ErrorStrategy::<String>::Retry(5));
  let mut error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::default(),
  );
  error.retries = 3;
  assert_eq!(producer.handle_error(&error), ErrorAction::Retry);
  drop(file);
}

#[tokio::test]
async fn test_file_producer_handle_error_retry_exceeds_limit() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "line 1").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();
  let producer = FileProducer::new(path).with_error_strategy(ErrorStrategy::<String>::Retry(5));
  let mut error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::default(),
  );
  error.retries = 5;
  assert_eq!(producer.handle_error(&error), ErrorAction::Stop);
  drop(file);
}

#[tokio::test]
async fn test_file_producer_create_error_context() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "line 1").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();
  let producer = FileProducer::new(path).with_name("test_producer".to_string());
  let context = producer.create_error_context(Some("test_item".to_string()));
  assert_eq!(context.item, Some("test_item".to_string()));
  assert_eq!(context.component_name, "test_producer");
  assert!(context.timestamp <= chrono::Utc::now());
  drop(file);
}

#[tokio::test]
async fn test_file_producer_create_error_context_no_item() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "line 1").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();
  let producer = FileProducer::new(path).with_name("test_producer".to_string());
  let context = producer.create_error_context(None);
  assert_eq!(context.item, None);
  assert_eq!(context.component_name, "test_producer");
  drop(file);
}

#[tokio::test]
async fn test_file_producer_create_error_context_default_name() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "line 1").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();
  let producer = FileProducer::new(path);
  let context = producer.create_error_context(None);
  assert_eq!(context.component_name, "file_producer");
  drop(file);
}

#[tokio::test]
async fn test_file_producer_component_info() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "line 1").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();
  let producer = FileProducer::new(path).with_name("test_producer".to_string());
  let info = producer.component_info();
  assert_eq!(info.name, "test_producer");
  assert_eq!(info.type_name, std::any::type_name::<FileProducer>());
  drop(file);
}

#[tokio::test]
async fn test_file_producer_component_info_default_name() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "line 1").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();
  let producer = FileProducer::new(path);
  let info = producer.component_info();
  assert_eq!(info.name, "file_producer");
  assert_eq!(info.type_name, std::any::type_name::<FileProducer>());
  drop(file);
}

#[tokio::test]
async fn test_file_producer_read_error_handling() {
  // Test that read errors are handled gracefully
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "line 1").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();
  drop(file); // Drop file to potentially cause read issues
  let mut producer = FileProducer::new(path);
  let stream = producer.produce();
  let result: Vec<String> = stream.collect().await;
  // Should handle error gracefully and return empty or partial results
  assert!(result.is_empty() || result.len() <= 1);
}

#[tokio::test]
async fn test_file_producer_multiline_file() {
  let mut file = NamedTempFile::new().unwrap();
  for i in 1..=100 {
    writeln!(file, "line {}", i).unwrap();
  }
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();
  let mut producer = FileProducer::new(path);
  let stream = producer.produce();
  let result: Vec<String> = stream.collect().await;
  assert_eq!(result.len(), 100);
  assert_eq!(result[0], "line 1");
  assert_eq!(result[99], "line 100");
  drop(file);
}

#[tokio::test]
async fn test_file_producer_output_trait() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "test").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();
  let producer = FileProducer::new(path);
  // Test that Output trait is implemented
  fn assert_output_trait<P: Output<Output = String>>(_producer: P) {}
  assert_output_trait(producer);
  drop(file);
}
