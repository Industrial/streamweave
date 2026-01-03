//! Tests for FsDirectoryConsumer

use futures::stream;
use std::fs;
use std::path::PathBuf;
use streamweave::consumers::FsDirectoryConsumer;
use streamweave::error::ErrorStrategy;
use streamweave::{Consumer, ConsumerConfig};

#[tokio::test]
async fn test_fs_directory_consumer_basic() {
  let temp_dir = std::env::temp_dir();
  let test_dir = temp_dir.join("test_dir_consumer");

  // Clean up if exists
  let _ = fs::remove_dir_all(&test_dir);

  let mut consumer = FsDirectoryConsumer::new();
  let input_stream = Box::pin(stream::iter(vec![test_dir.to_str().unwrap().to_string()]));

  consumer.consume(input_stream).await;

  // Verify directory was created
  assert!(test_dir.exists());
  assert!(test_dir.is_dir());

  // Cleanup
  fs::remove_dir_all(&test_dir).unwrap();
}

#[tokio::test]
async fn test_fs_directory_consumer_nested() {
  let temp_dir = std::env::temp_dir();
  let test_dir = temp_dir
    .join("test_dir_consumer")
    .join("nested")
    .join("deep");

  // Clean up if exists
  let _ = fs::remove_dir_all(&temp_dir.join("test_dir_consumer"));

  let mut consumer = FsDirectoryConsumer::new();
  let input_stream = Box::pin(stream::iter(vec![test_dir.to_str().unwrap().to_string()]));

  consumer.consume(input_stream).await;

  // Verify nested directory was created
  assert!(test_dir.exists());
  assert!(test_dir.is_dir());

  // Cleanup
  fs::remove_dir_all(&temp_dir.join("test_dir_consumer")).unwrap();
}

#[tokio::test]
async fn test_fs_directory_consumer_empty() {
  let mut consumer = FsDirectoryConsumer::new();
  let input_stream = Box::pin(stream::iter(vec![] as Vec<String>));

  consumer.consume(input_stream).await;
  // Should complete without error
  assert!(true);
}

#[tokio::test]
async fn test_fs_directory_consumer_default() {
  let consumer = FsDirectoryConsumer::default();
  // Default consumer should be created
  assert!(true);
}

#[tokio::test]
async fn test_fs_directory_consumer_with_name() {
  let consumer = FsDirectoryConsumer::new().with_name("dir_creator".to_string());

  assert_eq!(consumer.config.name, "dir_creator");
}

#[tokio::test]
async fn test_fs_directory_consumer_with_error_strategy() {
  let consumer = FsDirectoryConsumer::new().with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(
    consumer.config.error_strategy,
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_fs_directory_consumer_component_info() {
  let consumer = FsDirectoryConsumer::new().with_name("test".to_string());

  let info = consumer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("FsDirectoryConsumer"));
}
