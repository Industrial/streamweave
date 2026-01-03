//! Tests for FileProducer

use futures::StreamExt;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use streamweave::error::ErrorStrategy;
use streamweave::producers::FileProducer;
use streamweave::{Producer, ProducerConfig};

#[tokio::test]
async fn test_file_producer_basic() {
  let temp_dir = std::env::temp_dir();
  let file_path = temp_dir.join("test_file_producer.txt");

  // Create test file
  let mut file = fs::File::create(&file_path).unwrap();
  writeln!(file, "line1").unwrap();
  writeln!(file, "line2").unwrap();
  writeln!(file, "line3").unwrap();
  drop(file);

  let mut producer = FileProducer::new(file_path.to_str().unwrap().to_string());
  let mut stream = producer.produce();

  let mut results = Vec::new();
  while let Some(item) = stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 3);
  assert_eq!(results[0], "line1");
  assert_eq!(results[1], "line2");
  assert_eq!(results[2], "line3");

  // Cleanup
  let _ = fs::remove_file(&file_path);
}

#[tokio::test]
async fn test_file_producer_empty_file() {
  let temp_dir = std::env::temp_dir();
  let file_path = temp_dir.join("test_file_producer_empty.txt");

  // Create empty file
  fs::File::create(&file_path).unwrap();

  let mut producer = FileProducer::new(file_path.to_str().unwrap().to_string());
  let mut stream = producer.produce();

  assert!(stream.next().await.is_none());

  // Cleanup
  let _ = fs::remove_file(&file_path);
}

#[tokio::test]
async fn test_file_producer_nonexistent_file() {
  let producer = FileProducer::new("/nonexistent/path/file.txt".to_string());
  let mut stream = producer.produce();

  // Should handle gracefully (empty stream)
  assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn test_file_producer_with_name() {
  let producer = FileProducer::new("test.txt".to_string()).with_name("file_reader".to_string());

  let config = producer.get_config_impl();
  assert_eq!(config.name(), Some("file_reader"));
}

#[tokio::test]
async fn test_file_producer_with_error_strategy() {
  let producer = FileProducer::new("test.txt".to_string()).with_error_strategy(ErrorStrategy::Skip);

  let config = producer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_file_producer_component_info() {
  let producer = FileProducer::new("test.txt".to_string()).with_name("test".to_string());

  let info = producer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("FileProducer"));
}
