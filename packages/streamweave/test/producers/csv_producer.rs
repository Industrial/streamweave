//! Tests for CsvProducer

use futures::StreamExt;
use serde::Deserialize;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use streamweave::error::ErrorStrategy;
use streamweave::producers::CsvProducer;
use streamweave::{Producer, ProducerConfig};

#[derive(Debug, Deserialize, Clone, PartialEq)]
struct TestRecord {
  name: String,
  age: u32,
}

// Note: CsvReadConfig is not exported, so we'll test through the producer methods

#[tokio::test]
async fn test_csv_producer_basic() {
  let temp_dir = std::env::temp_dir();
  let file_path = temp_dir.join("test_csv_producer.csv");

  // Create test CSV file
  let mut file = fs::File::create(&file_path).unwrap();
  writeln!(file, "name,age").unwrap();
  writeln!(file, "Alice,30").unwrap();
  writeln!(file, "Bob,25").unwrap();
  drop(file);

  let mut producer = CsvProducer::<TestRecord>::new(file_path.to_str().unwrap());
  let mut stream = producer.produce();

  let mut results = Vec::new();
  while let Some(item) = stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 2);
  assert_eq!(results[0].name, "Alice");
  assert_eq!(results[0].age, 30);
  assert_eq!(results[1].name, "Bob");
  assert_eq!(results[1].age, 25);

  // Cleanup
  let _ = fs::remove_file(&file_path);
}

#[tokio::test]
async fn test_csv_producer_no_headers() {
  let temp_dir = std::env::temp_dir();
  let file_path = temp_dir.join("test_csv_producer_no_headers.csv");

  // Create CSV without header
  let mut file = fs::File::create(&file_path).unwrap();
  writeln!(file, "Alice,30").unwrap();
  writeln!(file, "Bob,25").unwrap();
  drop(file);

  let mut producer =
    CsvProducer::<TestRecord>::new(file_path.to_str().unwrap()).with_headers(false);
  let mut stream = producer.produce();

  let mut results = Vec::new();
  while let Some(item) = stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 2);

  // Cleanup
  let _ = fs::remove_file(&file_path);
}

#[tokio::test]
async fn test_csv_producer_with_name() {
  let producer = CsvProducer::<TestRecord>::new("test.csv").with_name("csv_reader".to_string());

  let config = producer.get_config_impl();
  assert_eq!(config.name(), Some("csv_reader"));
}

#[tokio::test]
async fn test_csv_producer_with_error_strategy() {
  let producer =
    CsvProducer::<TestRecord>::new("test.csv").with_error_strategy(ErrorStrategy::Skip);

  let config = producer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_csv_producer_component_info() {
  let producer = CsvProducer::<TestRecord>::new("test.csv").with_name("test".to_string());

  let info = producer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("CsvProducer"));
}

#[tokio::test]
async fn test_csv_producer_with_delimiter() {
  let temp_dir = std::env::temp_dir();
  let file_path = temp_dir.join("test_csv_producer_delimiter.csv");

  // Create CSV with semicolon delimiter
  let mut file = fs::File::create(&file_path).unwrap();
  writeln!(file, "name;age").unwrap();
  writeln!(file, "Alice;30").unwrap();
  drop(file);

  let mut producer =
    CsvProducer::<TestRecord>::new(file_path.to_str().unwrap()).with_delimiter(b';');
  let mut stream = producer.produce();

  let mut results = Vec::new();
  while let Some(item) = stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 1);
  assert_eq!(results[0].name, "Alice");

  // Cleanup
  let _ = fs::remove_file(&file_path);
}

#[tokio::test]
async fn test_csv_producer_with_trim() {
  let temp_dir = std::env::temp_dir();
  let file_path = temp_dir.join("test_csv_producer_trim.csv");

  // Create CSV with whitespace
  let mut file = fs::File::create(&file_path).unwrap();
  writeln!(file, "name,age").unwrap();
  writeln!(file, " Alice , 30 ").unwrap();
  drop(file);

  let mut producer = CsvProducer::<TestRecord>::new(file_path.to_str().unwrap()).with_trim(true);
  let mut stream = producer.produce();

  let mut results = Vec::new();
  while let Some(item) = stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 1);
  assert_eq!(results[0].name.trim(), "Alice");

  // Cleanup
  let _ = fs::remove_file(&file_path);
}
