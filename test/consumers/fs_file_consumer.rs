//! Tests for FsFileConsumer

use futures::stream;
use std::fs;
use std::path::PathBuf;
use streamweave::consumers::FsFileConsumer;
use streamweave::error::ErrorStrategy;
use streamweave::{Consumer, ConsumerConfig};

#[tokio::test]
async fn test_file_consumer_basic() {
  let temp_dir = std::env::temp_dir();
  let file_path = temp_dir.join("test_file_consumer.txt");

  // Clean up if exists
  let _ = fs::remove_file(&file_path);

  let mut consumer = FsFileConsumer::new(file_path.to_str().unwrap().to_string());
  let input_stream = Box::pin(stream::iter(vec![
    "line1".to_string(),
    "line2".to_string(),
    "line3".to_string(),
  ]));

  consumer.consume(input_stream).await;

  // Verify file contents
  let contents = fs::read_to_string(&file_path).unwrap();
  assert!(contents.contains("line1"));
  assert!(contents.contains("line2"));
  assert!(contents.contains("line3"));

  // Cleanup
  let _ = fs::remove_file(&file_path);
}

#[tokio::test]
async fn test_file_consumer_empty() {
  let temp_dir = std::env::temp_dir();
  let file_path = temp_dir.join("test_file_consumer_empty.txt");

  // Clean up if exists
  let _ = fs::remove_file(&file_path);

  let mut consumer = FsFileConsumer::new(file_path.to_str().unwrap().to_string());
  let input_stream = Box::pin(stream::iter(vec![] as Vec<String>));

  consumer.consume(input_stream).await;

  // File should exist but be empty
  assert!(file_path.exists());

  // Cleanup
  let _ = fs::remove_file(&file_path);
}

#[tokio::test]
async fn test_file_consumer_with_name() {
  let consumer = FsFileConsumer::new("test.txt".to_string()).with_name("file_writer".to_string());

  assert_eq!(consumer.config.name, "file_writer");
}

#[tokio::test]
async fn test_file_consumer_with_error_strategy() {
  let consumer =
    FsFileConsumer::new("test.txt".to_string()).with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(
    consumer.config.error_strategy,
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_file_consumer_component_info() {
  let consumer = FsFileConsumer::new("test.txt".to_string()).with_name("test".to_string());

  let info = consumer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("FsFileConsumer"));
}
