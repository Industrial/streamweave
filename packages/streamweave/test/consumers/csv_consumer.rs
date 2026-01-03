//! Tests for CsvConsumer

use futures::stream;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use streamweave::consumers::CsvConsumer;
use streamweave::error::ErrorStrategy;
use streamweave::{Consumer, ConsumerConfig};

#[derive(Debug, Serialize, Clone, PartialEq)]
struct TestRecord {
  name: String,
  age: u32,
}

#[tokio::test]
async fn test_csv_consumer_basic() {
  let temp_dir = std::env::temp_dir();
  let file_path = temp_dir.join("test_csv_consumer.csv");

  // Clean up if exists
  let _ = fs::remove_file(&file_path);

  let mut consumer = CsvConsumer::<TestRecord>::new(file_path.to_str().unwrap());
  let input_stream = Box::pin(stream::iter(vec![
    TestRecord {
      name: "Alice".to_string(),
      age: 30,
    },
    TestRecord {
      name: "Bob".to_string(),
      age: 25,
    },
  ]));

  consumer.consume(input_stream).await;

  // Verify file was created and has content
  assert!(file_path.exists());
  let contents = fs::read_to_string(&file_path).unwrap();
  assert!(contents.contains("name,age"));
  assert!(contents.contains("Alice"));
  assert!(contents.contains("Bob"));

  // Cleanup
  let _ = fs::remove_file(&file_path);
}

#[tokio::test]
async fn test_csv_consumer_no_headers() {
  let temp_dir = std::env::temp_dir();
  let file_path = temp_dir.join("test_csv_consumer_no_headers.csv");

  // Clean up if exists
  let _ = fs::remove_file(&file_path);

  let mut consumer =
    CsvConsumer::<TestRecord>::new(file_path.to_str().unwrap()).with_headers(false);
  let input_stream = Box::pin(stream::iter(vec![TestRecord {
    name: "Alice".to_string(),
    age: 30,
  }]));

  consumer.consume(input_stream).await;

  let contents = fs::read_to_string(&file_path).unwrap();
  assert!(!contents.contains("name,age")); // No header

  // Cleanup
  let _ = fs::remove_file(&file_path);
}

#[tokio::test]
async fn test_csv_consumer_empty() {
  let temp_dir = std::env::temp_dir();
  let file_path = temp_dir.join("test_csv_consumer_empty.csv");

  // Clean up if exists
  let _ = fs::remove_file(&file_path);

  let mut consumer = CsvConsumer::<TestRecord>::new(file_path.to_str().unwrap());
  let input_stream = Box::pin(stream::iter(vec![] as Vec<TestRecord>));

  consumer.consume(input_stream).await;

  // File should exist (created on first write attempt)
  assert!(file_path.exists());

  // Cleanup
  let _ = fs::remove_file(&file_path);
}

#[tokio::test]
async fn test_csv_consumer_with_name() {
  let consumer = CsvConsumer::<TestRecord>::new("test.csv").with_name("csv_writer".to_string());

  assert_eq!(consumer.config.name, "csv_writer");
}

#[tokio::test]
async fn test_csv_consumer_with_error_strategy() {
  let consumer =
    CsvConsumer::<TestRecord>::new("test.csv").with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(
    consumer.config.error_strategy,
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_csv_consumer_component_info() {
  let consumer = CsvConsumer::<TestRecord>::new("test.csv").with_name("test".to_string());

  let info = consumer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("CsvConsumer"));
}

#[tokio::test]
async fn test_csv_consumer_with_delimiter() {
  let temp_dir = std::env::temp_dir();
  let file_path = temp_dir.join("test_csv_consumer_delimiter.csv");

  // Clean up if exists
  let _ = fs::remove_file(&file_path);

  let mut consumer = CsvConsumer::<TestRecord>::new(&file_path).with_delimiter(b';');
  let input_stream = Box::pin(stream::iter(vec![TestRecord {
    name: "Alice".to_string(),
    age: 30,
  }]));

  consumer.consume(input_stream).await;

  let contents = fs::read_to_string(&file_path).unwrap();
  assert!(contents.contains(";"));

  // Cleanup
  let _ = fs::remove_file(&file_path);
}

#[tokio::test]
async fn test_csv_consumer_with_flush_on_write() {
  let temp_dir = std::env::temp_dir();
  let file_path = temp_dir.join("test_csv_consumer_flush.csv");

  // Clean up if exists
  let _ = fs::remove_file(&file_path);

  let mut consumer = CsvConsumer::<TestRecord>::new(&file_path).with_flush_on_write(true);
  let input_stream = Box::pin(stream::iter(vec![TestRecord {
    name: "Alice".to_string(),
    age: 30,
  }]));

  consumer.consume(input_stream).await;

  // File should exist
  assert!(file_path.exists());

  // Cleanup
  let _ = fs::remove_file(&file_path);
}
