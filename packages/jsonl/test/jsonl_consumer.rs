//! Tests for JsonlConsumer

use futures::{StreamExt, stream};
use proptest::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::BufRead;
use std::path::PathBuf;
use streamweave::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave::{Consumer, Input};
use streamweave_jsonl::{JsonlConsumer, JsonlWriteConfig};
use tempfile::NamedTempFile;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TestRecord {
  id: u32,
  name: String,
}

fn test_record_strategy() -> impl Strategy<Value = TestRecord> {
  (
    prop::num::u32::ANY,
    prop::string::string_regex("[a-zA-Z0-9 ]+").unwrap(),
  )
    .prop_map(|(id, name)| TestRecord { id, name })
}

async fn test_jsonl_consumer_basic_async(records: Vec<TestRecord>) {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let mut consumer = JsonlConsumer::new(&path);
  let input_stream = Box::pin(stream::iter(records.clone()));
  consumer.consume(input_stream).await;

  // Read and verify the file
  let file_content = std::fs::File::open(&path).unwrap();
  let reader = std::io::BufReader::new(file_content);
  let lines: Vec<TestRecord> = reader
    .lines()
    .map(|l| serde_json::from_str(&l.unwrap()).unwrap())
    .collect();

  assert_eq!(lines, records);
  drop(file);
}

proptest! {
  #[test]
  fn test_jsonl_consumer_new(path in "[a-zA-Z0-9_./-]+\\.jsonl") {
    let consumer = JsonlConsumer::<TestRecord>::new(path.clone());
    prop_assert_eq!(consumer.path(), &PathBuf::from(path));
  }

  #[test]
  fn test_jsonl_consumer_builder(
    path in "[a-zA-Z0-9_./-]+\\.jsonl",
    name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
    append in prop::bool::ANY,
    buffer_size in 1024usize..65536usize
  ) {
    let consumer = JsonlConsumer::<TestRecord>::new(path.clone())
      .with_name(name.clone())
      .with_append(append)
      .with_buffer_size(buffer_size);

    prop_assert_eq!(consumer.path(), &PathBuf::from(path));
    prop_assert_eq!(consumer.config.name, name);
    prop_assert_eq!(consumer.jsonl_config.append, append);
    prop_assert_eq!(consumer.jsonl_config.buffer_size, buffer_size);
  }

  #[test]
  fn test_jsonl_write_config_default(_ in prop::num::u8::ANY) {
    let config = JsonlWriteConfig::default();
    prop_assert!(!config.append);
    prop_assert!(!config.pretty);
    prop_assert_eq!(config.buffer_size, 8192);
  }

  #[test]
  fn test_jsonl_write_config_builder(
    append in prop::bool::ANY,
    buffer_size in 1024usize..65536usize
  ) {
    let config = JsonlWriteConfig::default()
      .with_append(append)
      .with_buffer_size(buffer_size);

    prop_assert_eq!(config.append, append);
    prop_assert_eq!(config.buffer_size, buffer_size);
  }

  #[test]
  fn test_jsonl_consumer_basic(
    records in prop::collection::vec(test_record_strategy(), 0..20)
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_jsonl_consumer_basic_async(records));
  }
}

async fn test_jsonl_consumer_append_async(records1: Vec<TestRecord>, records2: Vec<TestRecord>) {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  // Write first batch
  let mut consumer1 = JsonlConsumer::new(&path);
  let input_stream1 = Box::pin(stream::iter(records1.clone()));
  consumer1.consume(input_stream1).await;

  // Append second batch
  let mut consumer2 = JsonlConsumer::new(&path).with_append(true);
  let input_stream2 = Box::pin(stream::iter(records2.clone()));
  consumer2.consume(input_stream2).await;

  // Read and verify the file
  let file_content = std::fs::File::open(&path).unwrap();
  let reader = std::io::BufReader::new(file_content);
  let lines: Vec<TestRecord> = reader
    .lines()
    .map(|l| serde_json::from_str(&l.unwrap()).unwrap())
    .collect();

  let mut expected = records1;
  expected.extend(records2);
  assert_eq!(lines, expected);
  drop(file);
}

proptest! {
  #[test]
  fn test_jsonl_consumer_append(
    records1 in prop::collection::vec(test_record_strategy(), 0..10),
    records2 in prop::collection::vec(test_record_strategy(), 0..10)
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_jsonl_consumer_append_async(records1, records2));
  }
}

async fn test_jsonl_consumer_overwrite_async(records1: Vec<TestRecord>, records2: Vec<TestRecord>) {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  // Write first batch
  let mut consumer1 = JsonlConsumer::new(&path);
  let input_stream1 = Box::pin(stream::iter(records1));
  consumer1.consume(input_stream1).await;

  // Overwrite with second batch
  let mut consumer2 = JsonlConsumer::new(&path).with_append(false);
  let input_stream2 = Box::pin(stream::iter(records2.clone()));
  consumer2.consume(input_stream2).await;

  // Read and verify the file
  let file_content = std::fs::File::open(&path).unwrap();
  let reader = std::io::BufReader::new(file_content);
  let lines: Vec<TestRecord> = reader
    .lines()
    .map(|l| serde_json::from_str(&l.unwrap()).unwrap())
    .collect();

  assert_eq!(lines, records2);
  drop(file);
}

proptest! {
  #[test]
  fn test_jsonl_consumer_overwrite(
    records1 in prop::collection::vec(test_record_strategy(), 0..10),
    records2 in prop::collection::vec(test_record_strategy(), 0..10)
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_jsonl_consumer_overwrite_async(records1, records2));
  }
}

async fn test_jsonl_consumer_empty_stream_async() {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let mut consumer = JsonlConsumer::<TestRecord>::new(&path);
  let input_stream = Box::pin(stream::iter(Vec::<TestRecord>::new()));
  consumer.consume(input_stream).await;

  // Read and verify the file is empty
  let content = std::fs::read_to_string(&path).unwrap();
  assert!(content.is_empty());
  drop(file);
}

proptest! {
  #[test]
  fn test_jsonl_consumer_empty_stream(_ in prop::num::u8::ANY) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_jsonl_consumer_empty_stream_async());
  }

  #[test]
  fn test_jsonl_consumer_component_info(
    name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
  ) {
    let consumer = JsonlConsumer::<TestRecord>::new("test.jsonl").with_name(name.clone());
    let info = consumer.component_info();
    prop_assert_eq!(info.name, name);
    prop_assert_eq!(
      info.type_name,
      std::any::type_name::<JsonlConsumer<TestRecord>>()
    );
  }

  #[test]
  fn test_jsonl_consumer_create_error_context(
    record in test_record_strategy(),
    name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
  ) {
    let consumer = JsonlConsumer::<TestRecord>::new("test.jsonl").with_name(name.clone());
    let ctx = consumer.create_error_context(Some(record.clone()));
    prop_assert_eq!(ctx.component_name, name);
    prop_assert_eq!(ctx.item, Some(record));
  }
}

// Input trait tests

#[test]
fn test_jsonl_consumer_input_trait_implementation() {
  fn assert_input_trait<T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static>(
    _consumer: JsonlConsumer<T>,
  ) where
    JsonlConsumer<T>: Input<Input = T>,
  {
  }

  let consumer = JsonlConsumer::<TestRecord>::new("test.jsonl");
  assert_input_trait(consumer);
}

#[tokio::test]
async fn test_jsonl_consumer_error_strategy_retry() {
  // Note: Retry is not fully supported for serialization/write errors, so it should skip
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let mut consumer =
    JsonlConsumer::<TestRecord>::new(&path).with_error_strategy(ErrorStrategy::Retry(3));
  let input_stream = Box::pin(stream::iter(vec![TestRecord {
    id: 1,
    name: "Alice".to_string(),
  }]));
  consumer.consume(input_stream).await;

  // Should write successfully
  let content = std::fs::read_to_string(&path).unwrap();
  assert!(!content.is_empty());
  drop(file);
}

#[tokio::test]
async fn test_jsonl_consumer_error_strategy_custom() {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let custom_handler = |_error: &StreamError<TestRecord>| ErrorAction::Skip;
  let mut consumer = JsonlConsumer::<TestRecord>::new(&path)
    .with_error_strategy(ErrorStrategy::new_custom(custom_handler));
  let input_stream = Box::pin(stream::iter(vec![TestRecord {
    id: 1,
    name: "Alice".to_string(),
  }]));
  consumer.consume(input_stream).await;

  let content = std::fs::read_to_string(&path).unwrap();
  assert!(!content.is_empty());
  drop(file);
}

#[tokio::test]
async fn test_jsonl_consumer_error_strategy_stop() {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let mut consumer =
    JsonlConsumer::<TestRecord>::new(&path).with_error_strategy(ErrorStrategy::Stop);
  let input_stream = Box::pin(stream::iter(vec![TestRecord {
    id: 1,
    name: "Alice".to_string(),
  }]));
  consumer.consume(input_stream).await;

  let content = std::fs::read_to_string(&path).unwrap();
  assert!(!content.is_empty());
  drop(file);
}

#[test]
fn test_jsonl_consumer_config_methods() {
  let mut consumer = JsonlConsumer::<TestRecord>::new("test.jsonl");
  let config = consumer.config().clone();
  assert_eq!(config.name, String::new());

  let config_mut = consumer.config_mut();
  config_mut.name = "test".to_string();
  assert_eq!(consumer.config().name, "test".to_string());

  let new_config = streamweave::ConsumerConfig::default();
  consumer.set_config(new_config.clone());
  assert_eq!(consumer.config().name, new_config.name);
}

#[test]
fn test_jsonl_consumer_handle_error() {
  let consumer =
    JsonlConsumer::<TestRecord>::new("test.jsonl").with_error_strategy(ErrorStrategy::Skip);
  let error = StreamError::new(
    Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test")),
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "test".to_string(),
    },
    ComponentInfo {
      name: "test".to_string(),
      type_name: "test".to_string(),
    },
  );
  assert_eq!(consumer.handle_error(&error), ErrorAction::Skip);

  let consumer_stop =
    JsonlConsumer::<TestRecord>::new("test.jsonl").with_error_strategy(ErrorStrategy::Stop);
  assert_eq!(consumer_stop.handle_error(&error), ErrorAction::Stop);

  let consumer_retry =
    JsonlConsumer::<TestRecord>::new("test.jsonl").with_error_strategy(ErrorStrategy::Retry(3));
  let mut error_retry = error.clone();
  error_retry.retries = 0;
  assert_eq!(
    consumer_retry.handle_error(&error_retry),
    ErrorAction::Retry
  );

  let mut error_retry_max = error.clone();
  error_retry_max.retries = 3;
  assert_eq!(
    consumer_retry.handle_error(&error_retry_max),
    ErrorAction::Stop
  );
}

#[test]
fn test_jsonl_consumer_empty_name() {
  let consumer = JsonlConsumer::<TestRecord>::new("test.jsonl");
  let info = consumer.component_info();
  assert_eq!(info.name, "jsonl_consumer");
}

#[test]
fn test_jsonl_write_config_pretty() {
  let config = JsonlWriteConfig::default();
  assert!(!config.pretty);
  // Note: pretty is not used in the implementation, but we test the default
}

#[test]
fn test_jsonl_consumer_with_pretty() {
  // Test that pretty config exists (even if not used)
  let consumer = JsonlConsumer::<TestRecord>::new("test.jsonl");
  assert!(!consumer.jsonl_config.pretty);
}
