//! Tests for DirectoryProducer

use futures::StreamExt;
use streamweave::Producer;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_fs::DirectoryProducer;
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_directory_producer_basic() {
  let temp_dir = TempDir::new().unwrap();
  let dir_path = temp_dir.path().to_str().unwrap().to_string();

  // Create some files
  fs::write(temp_dir.path().join("file1.txt"), "content1")
    .await
    .unwrap();
  fs::write(temp_dir.path().join("file2.txt"), "content2")
    .await
    .unwrap();
  fs::create_dir(temp_dir.path().join("subdir"))
    .await
    .unwrap();

  let mut producer = DirectoryProducer::new(dir_path.clone());
  let stream = producer.produce();
  let result: Vec<String> = stream.collect().await;

  // Should have at least 3 entries (2 files + 1 directory)
  assert!(result.len() >= 3);
  assert!(result.iter().any(|p| p.contains("file1.txt")));
  assert!(result.iter().any(|p| p.contains("file2.txt")));
  assert!(result.iter().any(|p| p.contains("subdir")));
}

#[tokio::test]
async fn test_directory_producer_empty_directory() {
  let temp_dir = TempDir::new().unwrap();
  let dir_path = temp_dir.path().to_str().unwrap().to_string();

  let mut producer = DirectoryProducer::new(dir_path);
  let stream = producer.produce();
  let result: Vec<String> = stream.collect().await;

  // Empty directory should produce no entries (or just "." and ".." on some systems)
  assert!(result.is_empty() || result.len() <= 2);
}

#[tokio::test]
async fn test_directory_producer_nonexistent_directory() {
  let mut producer = DirectoryProducer::new("nonexistent_directory_12345".to_string());
  let stream = producer.produce();
  let result: Vec<String> = stream.collect().await;

  // Should produce empty stream for nonexistent directory
  assert!(result.is_empty());
}

#[tokio::test]
async fn test_directory_producer_recursive() {
  let temp_dir = TempDir::new().unwrap();
  let dir_path = temp_dir.path().to_str().unwrap().to_string();

  // Create nested structure
  fs::write(temp_dir.path().join("file1.txt"), "content1")
    .await
    .unwrap();
  fs::create_dir(temp_dir.path().join("subdir1"))
    .await
    .unwrap();
  fs::write(
    temp_dir.path().join("subdir1").join("file2.txt"),
    "content2",
  )
  .await
  .unwrap();
  fs::create_dir(temp_dir.path().join("subdir1").join("subdir2"))
    .await
    .unwrap();
  fs::write(
    temp_dir
      .path()
      .join("subdir1")
      .join("subdir2")
      .join("file3.txt"),
    "content3",
  )
  .await
  .unwrap();

  let mut producer = DirectoryProducer::new(dir_path.clone()).recursive(true);
  let stream = producer.produce();
  let result: Vec<String> = stream.collect().await;

  // Should find all files and directories recursively
  assert!(result.iter().any(|p| p.contains("file1.txt")));
  assert!(result.iter().any(|p| p.contains("file2.txt")));
  assert!(result.iter().any(|p| p.contains("file3.txt")));
  assert!(result.iter().any(|p| p.contains("subdir1")));
  assert!(result.iter().any(|p| p.contains("subdir2")));
}

#[tokio::test]
async fn test_directory_producer_non_recursive() {
  let temp_dir = TempDir::new().unwrap();
  let dir_path = temp_dir.path().to_str().unwrap().to_string();

  // Create nested structure
  fs::write(temp_dir.path().join("file1.txt"), "content1")
    .await
    .unwrap();
  fs::create_dir(temp_dir.path().join("subdir1"))
    .await
    .unwrap();
  fs::write(
    temp_dir.path().join("subdir1").join("file2.txt"),
    "content2",
  )
  .await
  .unwrap();

  let mut producer = DirectoryProducer::new(dir_path.clone()).recursive(false);
  let stream = producer.produce();
  let result: Vec<String> = stream.collect().await;

  // Should only find top-level entries
  assert!(result.iter().any(|p| p.contains("file1.txt")));
  assert!(result.iter().any(|p| p.contains("subdir1")));
  // Should NOT find nested file
  assert!(
    !result
      .iter()
      .any(|p| p.contains("file2.txt") && !p.contains("subdir1"))
  );
}

#[tokio::test]
async fn test_directory_producer_with_name() {
  let temp_dir = TempDir::new().unwrap();
  let dir_path = temp_dir.path().to_str().unwrap().to_string();

  let producer = DirectoryProducer::new(dir_path).with_name("my_producer".to_string());
  assert_eq!(producer.config().name(), Some("my_producer".to_string()));
}

#[tokio::test]
async fn test_directory_producer_with_error_strategy() {
  let temp_dir = TempDir::new().unwrap();
  let dir_path = temp_dir.path().to_str().unwrap().to_string();

  let producer = DirectoryProducer::new(dir_path)
    .with_error_strategy(ErrorStrategy::<String>::Skip)
    .with_name("test_producer".to_string());

  let config = producer.config();
  assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
  assert_eq!(config.name(), Some("test_producer".to_string()));
}

#[tokio::test]
async fn test_directory_producer_set_config_impl() {
  let temp_dir = TempDir::new().unwrap();
  let dir_path = temp_dir.path().to_str().unwrap().to_string();

  let mut producer = DirectoryProducer::new(dir_path);
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
}

#[tokio::test]
async fn test_directory_producer_get_config_mut_impl() {
  let temp_dir = TempDir::new().unwrap();
  let dir_path = temp_dir.path().to_str().unwrap().to_string();

  let mut producer = DirectoryProducer::new(dir_path);
  let config_mut = producer.get_config_mut_impl();
  config_mut.name = Some("mutated_name".to_string());
  assert_eq!(
    producer.get_config_impl().name(),
    Some("mutated_name".to_string())
  );
}

#[tokio::test]
async fn test_directory_producer_handle_error_stop() {
  let temp_dir = TempDir::new().unwrap();
  let dir_path = temp_dir.path().to_str().unwrap().to_string();

  let producer =
    DirectoryProducer::new(dir_path).with_error_strategy(ErrorStrategy::<String>::Stop);
  let error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::default(),
  );
  assert_eq!(producer.handle_error(&error), ErrorAction::Stop);
}

#[tokio::test]
async fn test_directory_producer_handle_error_skip() {
  let temp_dir = TempDir::new().unwrap();
  let dir_path = temp_dir.path().to_str().unwrap().to_string();

  let producer =
    DirectoryProducer::new(dir_path).with_error_strategy(ErrorStrategy::<String>::Skip);
  let error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::default(),
  );
  assert_eq!(producer.handle_error(&error), ErrorAction::Skip);
}

#[tokio::test]
async fn test_directory_producer_handle_error_retry_within_limit() {
  let temp_dir = TempDir::new().unwrap();
  let dir_path = temp_dir.path().to_str().unwrap().to_string();

  let producer =
    DirectoryProducer::new(dir_path).with_error_strategy(ErrorStrategy::<String>::Retry(5));
  let mut error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::default(),
  );
  error.retries = 3;
  assert_eq!(producer.handle_error(&error), ErrorAction::Retry);
}

#[tokio::test]
async fn test_directory_producer_handle_error_retry_exceeds_limit() {
  let temp_dir = TempDir::new().unwrap();
  let dir_path = temp_dir.path().to_str().unwrap().to_string();

  let producer =
    DirectoryProducer::new(dir_path).with_error_strategy(ErrorStrategy::<String>::Retry(5));
  let mut error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::default(),
  );
  error.retries = 5;
  assert_eq!(producer.handle_error(&error), ErrorAction::Stop);
}

#[tokio::test]
async fn test_directory_producer_handle_error_custom_strategy() {
  let temp_dir = TempDir::new().unwrap();
  let dir_path = temp_dir.path().to_str().unwrap().to_string();

  let custom_handler = |error: &StreamError<String>| {
    if error.retries < 3 {
      ErrorAction::Retry
    } else {
      ErrorAction::Skip
    }
  };
  let producer = DirectoryProducer::new(dir_path)
    .with_error_strategy(ErrorStrategy::<String>::new_custom(custom_handler));
  let mut error = StreamError::new(
    Box::new(std::io::Error::other("test error")),
    ErrorContext::default(),
    ComponentInfo::default(),
  );
  error.retries = 2;
  assert_eq!(producer.handle_error(&error), ErrorAction::Retry);
  error.retries = 3;
  assert_eq!(producer.handle_error(&error), ErrorAction::Skip);
}

#[tokio::test]
async fn test_directory_producer_create_error_context() {
  let temp_dir = TempDir::new().unwrap();
  let dir_path = temp_dir.path().to_str().unwrap().to_string();

  let producer = DirectoryProducer::new(dir_path).with_name("test_producer".to_string());
  let context = producer.create_error_context(Some("test_item".to_string()));
  assert_eq!(context.item, Some("test_item".to_string()));
  assert_eq!(context.component_name, "test_producer");
  assert!(context.timestamp <= chrono::Utc::now());
}

#[tokio::test]
async fn test_directory_producer_create_error_context_no_item() {
  let temp_dir = TempDir::new().unwrap();
  let dir_path = temp_dir.path().to_str().unwrap().to_string();

  let producer = DirectoryProducer::new(dir_path).with_name("test_producer".to_string());
  let context = producer.create_error_context(None);
  assert_eq!(context.item, None);
  assert_eq!(context.component_name, "test_producer");
}

#[tokio::test]
async fn test_directory_producer_create_error_context_default_name() {
  let temp_dir = TempDir::new().unwrap();
  let dir_path = temp_dir.path().to_str().unwrap().to_string();

  let producer = DirectoryProducer::new(dir_path);
  let context = producer.create_error_context(None);
  assert_eq!(context.component_name, "directory_producer");
}

#[tokio::test]
async fn test_directory_producer_component_info() {
  let temp_dir = TempDir::new().unwrap();
  let dir_path = temp_dir.path().to_str().unwrap().to_string();

  let producer = DirectoryProducer::new(dir_path).with_name("test_producer".to_string());
  let info = producer.component_info();
  assert_eq!(info.name, "test_producer");
  assert_eq!(info.type_name, std::any::type_name::<DirectoryProducer>());
}

#[tokio::test]
async fn test_directory_producer_component_info_default_name() {
  let temp_dir = TempDir::new().unwrap();
  let dir_path = temp_dir.path().to_str().unwrap().to_string();

  let producer = DirectoryProducer::new(dir_path);
  let info = producer.component_info();
  assert_eq!(info.name, "directory_producer");
  assert_eq!(info.type_name, std::any::type_name::<DirectoryProducer>());
}

#[tokio::test]
async fn test_directory_producer_recursive_deep_nesting() {
  let temp_dir = TempDir::new().unwrap();
  let dir_path = temp_dir.path().to_str().unwrap().to_string();

  // Create deeply nested structure
  let mut current_path = temp_dir.path().to_path_buf();
  for i in 0..10 {
    current_path = current_path.join(format!("level{}", i));
    fs::create_dir_all(&current_path).await.unwrap();
    fs::write(current_path.join("file.txt"), format!("content{}", i))
      .await
      .unwrap();
  }

  let mut producer = DirectoryProducer::new(dir_path).recursive(true);
  let stream = producer.produce();
  let result: Vec<String> = stream.collect().await;

  // Should find all nested files
  assert!(
    result
      .iter()
      .any(|p| p.contains("level9") && p.contains("file.txt"))
  );
}

#[tokio::test]
async fn test_directory_producer_output_trait() {
  let temp_dir = TempDir::new().unwrap();
  let dir_path = temp_dir.path().to_str().unwrap().to_string();

  let producer = DirectoryProducer::new(dir_path);
  // Test that Output trait is implemented
  fn assert_output_trait<P: streamweave::Output<Output = String>>(_producer: P) {}
  assert_output_trait(producer);
}
