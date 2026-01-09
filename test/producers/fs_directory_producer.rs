//! Tests for FsDirectoryProducer

use futures::StreamExt;
use std::fs;
use std::path::PathBuf;
use streamweave::error::ErrorStrategy;
use streamweave::producers::FsDirectoryProducer;
use streamweave::{Producer, ProducerConfig};

#[tokio::test]
async fn test_directory_producer_basic() {
  let temp_dir = std::env::temp_dir();
  let test_dir = temp_dir.join("test_dir_producer");

  // Create test directory with files
  fs::create_dir_all(&test_dir).unwrap();
  fs::File::create(test_dir.join("file1.txt")).unwrap();
  fs::File::create(test_dir.join("file2.txt")).unwrap();

  let mut producer = FsDirectoryProducer::new(test_dir.to_str().unwrap().to_string());
  let mut stream = producer.produce();

  let mut results = Vec::new();
  while let Some(item) = stream.next().await {
    results.push(item);
  }

  // Should have at least the files we created
  assert!(!results.is_empty());

  // Cleanup
  fs::remove_dir_all(&test_dir).unwrap();
}

#[tokio::test]
async fn test_directory_producer_recursive() {
  let temp_dir = std::env::temp_dir();
  let test_dir = temp_dir.join("test_dir_producer_recursive");
  let sub_dir = test_dir.join("subdir");

  // Create nested structure
  fs::create_dir_all(&sub_dir).unwrap();
  fs::File::create(test_dir.join("file1.txt")).unwrap();
  fs::File::create(sub_dir.join("file2.txt")).unwrap();

  let mut producer =
    FsDirectoryProducer::new(test_dir.to_str().unwrap().to_string()).recursive(true);
  let mut stream = producer.produce();

  let mut results = Vec::new();
  while let Some(item) = stream.next().await {
    results.push(item);
  }

  // Should have files from both directories
  assert!(results.len() >= 2);

  // Cleanup
  fs::remove_dir_all(&test_dir).unwrap();
}

#[tokio::test]
async fn test_directory_producer_nonexistent() {
  let producer = FsDirectoryProducer::new("/nonexistent/directory".to_string());
  let mut stream = producer.produce();

  // Should handle gracefully (empty stream)
  assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn test_directory_producer_with_name() {
  let producer = FsDirectoryProducer::new("/tmp".to_string()).with_name("dir_reader".to_string());

  let config = producer.get_config_impl();
  assert_eq!(config.name(), Some("dir_reader"));
}

#[tokio::test]
async fn test_directory_producer_with_error_strategy() {
  let producer =
    FsDirectoryProducer::new("/tmp".to_string()).with_error_strategy(ErrorStrategy::Skip);

  let config = producer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_directory_producer_component_info() {
  let producer = FsDirectoryProducer::new("/tmp".to_string()).with_name("test".to_string());

  let info = producer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("FsDirectoryProducer"));
}
