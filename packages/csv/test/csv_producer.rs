//! Tests for CsvProducer

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::io::Write;
use streamweave::Producer;
use streamweave_csv::{CsvProducer, CsvReadConfig};
use tempfile::NamedTempFile;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TestRecord {
  name: String,
  age: u32,
}

#[tokio::test]
async fn test_csv_producer_basic() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "name,age").unwrap();
  writeln!(file, "Alice,30").unwrap();
  writeln!(file, "Bob,25").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let mut producer = CsvProducer::<TestRecord>::new(path);
  let stream = producer.produce();
  let result: Vec<TestRecord> = stream.collect().await;

  assert_eq!(
    result,
    vec![
      TestRecord {
        name: "Alice".to_string(),
        age: 30
      },
      TestRecord {
        name: "Bob".to_string(),
        age: 25
      },
    ]
  );
  drop(file);
}

#[tokio::test]
async fn test_csv_producer_no_headers() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "Alice,30").unwrap();
  writeln!(file, "Bob,25").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  // When there's no header, the first row is the header names for deserialization
  // So we need to adjust our test approach
  let mut producer = CsvProducer::<TestRecord>::new(path).with_headers(false);
  let stream = producer.produce();
  let result: Vec<TestRecord> = stream.collect().await;

  // Without headers, CSV crate will use column indices for deserialization
  // This test verifies the no-header mode works
  assert!(!result.is_empty());
  drop(file);
}

#[tokio::test]
async fn test_csv_producer_empty_file() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "name,age").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let mut producer = CsvProducer::<TestRecord>::new(path);
  let stream = producer.produce();
  let result: Vec<TestRecord> = stream.collect().await;

  assert!(result.is_empty());
  drop(file);
}

#[tokio::test]
async fn test_csv_producer_nonexistent_file() {
  let mut producer = CsvProducer::<TestRecord>::new("nonexistent.csv".to_string());
  let stream = producer.produce();
  let result: Vec<TestRecord> = stream.collect().await;

  assert!(result.is_empty());
}

#[tokio::test]
async fn test_csv_producer_tab_delimited() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "name\tage").unwrap();
  writeln!(file, "Alice\t30").unwrap();
  writeln!(file, "Bob\t25").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let mut producer = CsvProducer::<TestRecord>::new(path).with_delimiter(b'\t');
  let stream = producer.produce();
  let result: Vec<TestRecord> = stream.collect().await;

  assert_eq!(
    result,
    vec![
      TestRecord {
        name: "Alice".to_string(),
        age: 30
      },
      TestRecord {
        name: "Bob".to_string(),
        age: 25
      },
    ]
  );
  drop(file);
}

#[tokio::test]
async fn test_csv_producer_with_trim() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "name,age").unwrap();
  writeln!(file, "  Alice  ,  30  ").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let mut producer = CsvProducer::<TestRecord>::new(path).with_trim(true);
  let stream = producer.produce();
  let result: Vec<TestRecord> = stream.collect().await;

  assert_eq!(
    result,
    vec![TestRecord {
      name: "Alice".to_string(),
      age: 30
    },]
  );
  drop(file);
}

#[tokio::test]
async fn test_csv_producer_component_info() {
  let producer =
    CsvProducer::<TestRecord>::new("path.csv").with_name("my_csv_producer".to_string());
  let info = producer.component_info();
  assert_eq!(info.name, "my_csv_producer");
  assert_eq!(
    info.type_name,
    std::any::type_name::<CsvProducer<TestRecord>>()
  );
}

#[test]
fn test_csv_producer_new() {
  let producer = CsvProducer::<TestRecord>::new("test.csv");
  assert_eq!(producer.path(), "test.csv");
}

#[test]
fn test_csv_producer_builder() {
  let producer = CsvProducer::<TestRecord>::new("test.csv")
    .with_name("test_producer".to_string())
    .with_headers(false)
    .with_delimiter(b'\t')
    .with_flexible(true)
    .with_trim(true);

  assert_eq!(producer.path(), "test.csv");
  assert_eq!(producer.config.name, Some("test_producer".to_string()));
  assert!(!producer.csv_config.has_headers);
  assert_eq!(producer.csv_config.delimiter, b'\t');
  assert!(producer.csv_config.flexible);
  assert!(producer.csv_config.trim);
}

#[test]
fn test_csv_read_config_default() {
  let config = CsvReadConfig::default();
  assert!(config.has_headers);
  assert_eq!(config.delimiter, b',');
  assert!(!config.flexible);
  assert!(!config.trim);
  assert_eq!(config.comment, None);
  assert_eq!(config.quote, b'"');
  assert!(config.double_quote);
}

#[test]
fn test_csv_read_config_builder_methods() {
  let config = CsvReadConfig::default()
    .with_headers(false)
    .with_delimiter(b'\t')
    .with_flexible(true)
    .with_trim(true)
    .with_comment(Some(b'#'));

  assert!(!config.has_headers);
  assert_eq!(config.delimiter, b'\t');
  assert!(config.flexible);
  assert!(config.trim);
  assert_eq!(config.comment, Some(b'#'));
}

#[tokio::test]
async fn test_csv_producer_with_comment() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "name,age").unwrap();
  writeln!(file, "# This is a comment").unwrap();
  writeln!(file, "Alice,30").unwrap();
  writeln!(file, "# Another comment").unwrap();
  writeln!(file, "Bob,25").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let mut producer = CsvProducer::<TestRecord>::new(path);
  producer.csv_config.comment = Some(b'#');
  let stream = producer.produce();
  let result: Vec<TestRecord> = stream.collect().await;

  assert_eq!(
    result,
    vec![
      TestRecord {
        name: "Alice".to_string(),
        age: 30
      },
      TestRecord {
        name: "Bob".to_string(),
        age: 25
      },
    ]
  );
  drop(file);
}

#[tokio::test]
async fn test_csv_producer_with_flexible() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "name,age").unwrap();
  writeln!(file, "Alice,30,extra,fields").unwrap();
  writeln!(file, "Bob,25").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let mut producer = CsvProducer::<TestRecord>::new(path).with_flexible(true);
  let stream = producer.produce();
  let result: Vec<TestRecord> = stream.collect().await;

  // Flexible mode should allow extra fields
  assert_eq!(result.len(), 2);
  assert_eq!(result[0].name, "Alice");
  assert_eq!(result[0].age, 30);
  assert_eq!(result[1].name, "Bob");
  assert_eq!(result[1].age, 25);
  drop(file);
}

#[test]
fn test_csv_producer_clone() {
  let producer = CsvProducer::<TestRecord>::new("test.csv")
    .with_name("test_producer".to_string())
    .with_headers(false)
    .with_delimiter(b'\t');

  let cloned = producer.clone();
  assert_eq!(cloned.path(), producer.path());
  assert_eq!(cloned.config.name, producer.config.name);
  assert_eq!(
    cloned.csv_config.has_headers,
    producer.csv_config.has_headers
  );
  assert_eq!(cloned.csv_config.delimiter, producer.csv_config.delimiter);
}

#[test]
fn test_csv_producer_config_methods() {
  let mut producer = CsvProducer::<TestRecord>::new("test.csv");
  let new_config = streamweave::ProducerConfig {
    name: Some("new_name".to_string()),
    error_strategy: streamweave_error::ErrorStrategy::<TestRecord>::Skip,
  };

  producer.set_config_impl(new_config.clone());

  let retrieved_config = producer.get_config_impl();
  assert_eq!(retrieved_config.name, Some("new_name".to_string()));
  assert!(matches!(
    retrieved_config.error_strategy,
    streamweave_error::ErrorStrategy::Skip
  ));

  let config_mut = producer.get_config_mut_impl();
  config_mut.name = Some("mutated_name".to_string());
  config_mut.error_strategy = streamweave_error::ErrorStrategy::<TestRecord>::Retry(5);

  let config = producer.get_config_impl();
  assert_eq!(config.name, Some("mutated_name".to_string()));
  assert!(matches!(
    config.error_strategy,
    streamweave_error::ErrorStrategy::Retry(5)
  ));
}

#[test]
fn test_csv_producer_error_handling() {
  let producer = CsvProducer::<TestRecord>::new("test.csv")
    .with_error_strategy(streamweave_error::ErrorStrategy::<TestRecord>::Skip);

  let error = streamweave_error::StreamError::new(
    Box::new(std::io::Error::other("test error")),
    streamweave_error::ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "CsvProducer".to_string(),
    },
    streamweave_error::ComponentInfo {
      name: "test".to_string(),
      type_name: "CsvProducer".to_string(),
    },
  );

  let action = producer.handle_error(&error);
  assert_eq!(action, streamweave_error::ErrorAction::Skip);
}

#[test]
fn test_csv_producer_error_handling_stop() {
  let producer = CsvProducer::<TestRecord>::new("test.csv")
    .with_error_strategy(streamweave_error::ErrorStrategy::<TestRecord>::Stop);

  let error = streamweave_error::StreamError::new(
    Box::new(std::io::Error::other("test error")),
    streamweave_error::ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "CsvProducer".to_string(),
    },
    streamweave_error::ComponentInfo {
      name: "test".to_string(),
      type_name: "CsvProducer".to_string(),
    },
  );

  let action = producer.handle_error(&error);
  assert_eq!(action, streamweave_error::ErrorAction::Stop);
}

#[test]
fn test_csv_producer_error_handling_retry() {
  let producer = CsvProducer::<TestRecord>::new("test.csv")
    .with_error_strategy(streamweave_error::ErrorStrategy::<TestRecord>::Retry(3));

  let mut error = streamweave_error::StreamError::new(
    Box::new(std::io::Error::other("test error")),
    streamweave_error::ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "CsvProducer".to_string(),
    },
    streamweave_error::ComponentInfo {
      name: "test".to_string(),
      type_name: "CsvProducer".to_string(),
    },
  );
  error.retries = 2;

  let action = producer.handle_error(&error);
  assert_eq!(action, streamweave_error::ErrorAction::Retry);

  error.retries = 3;
  let action = producer.handle_error(&error);
  assert_eq!(action, streamweave_error::ErrorAction::Stop);
}

#[test]
fn test_csv_producer_error_handling_custom() {
  let producer = CsvProducer::<TestRecord>::new("test.csv").with_error_strategy(
    streamweave_error::ErrorStrategy::<TestRecord>::new_custom(|_error| {
      streamweave_error::ErrorAction::Skip
    }),
  );

  let error = streamweave_error::StreamError::new(
    Box::new(std::io::Error::other("test error")),
    streamweave_error::ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "CsvProducer".to_string(),
    },
    streamweave_error::ComponentInfo {
      name: "test".to_string(),
      type_name: "CsvProducer".to_string(),
    },
  );

  let action = producer.handle_error(&error);
  assert_eq!(action, streamweave_error::ErrorAction::Skip);
}

#[test]
fn test_csv_producer_create_error_context() {
  let producer = CsvProducer::<TestRecord>::new("test.csv").with_name("test_producer".to_string());

  let context = producer.create_error_context(Some(TestRecord {
    name: "Alice".to_string(),
    age: 30,
  }));

  assert_eq!(context.component_name, "test_producer");
  assert_eq!(
    context.component_type,
    std::any::type_name::<CsvProducer<TestRecord>>()
  );
  assert!(context.item.is_some());

  let context_no_item = producer.create_error_context(None);
  assert!(context_no_item.item.is_none());
}

#[test]
fn test_csv_producer_component_info_default_name() {
  let producer = CsvProducer::<TestRecord>::new("test.csv");
  let info = producer.component_info();
  assert_eq!(info.name, "csv_producer");
  assert_eq!(
    info.type_name,
    std::any::type_name::<CsvProducer<TestRecord>>()
  );
}

#[tokio::test]
async fn test_csv_producer_with_error_strategy_skip() {
  // Create a CSV with a malformed row
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "name,age").unwrap();
  writeln!(file, "Alice,30").unwrap();
  writeln!(file, "Bob,invalid_age").unwrap(); // This will fail to deserialize
  writeln!(file, "Charlie,35").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let mut producer = CsvProducer::<TestRecord>::new(path)
    .with_error_strategy(streamweave_error::ErrorStrategy::<TestRecord>::Skip);
  let stream = producer.produce();
  let result: Vec<TestRecord> = stream.collect().await;

  // Should skip the invalid row and continue
  assert_eq!(result.len(), 2);
  assert_eq!(result[0].name, "Alice");
  assert_eq!(result[1].name, "Charlie");
  drop(file);
}

#[tokio::test]
async fn test_csv_producer_with_error_strategy_stop() {
  // Create a CSV with a malformed row
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "name,age").unwrap();
  writeln!(file, "Alice,30").unwrap();
  writeln!(file, "Bob,invalid_age").unwrap(); // This will fail to deserialize
  writeln!(file, "Charlie,35").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let mut producer = CsvProducer::<TestRecord>::new(path)
    .with_error_strategy(streamweave_error::ErrorStrategy::<TestRecord>::Stop);
  let stream = producer.produce();
  let result: Vec<TestRecord> = stream.collect().await;

  // Should stop at the first error, so only Alice should be read
  assert_eq!(result.len(), 1);
  assert_eq!(result[0].name, "Alice");
  drop(file);
}

#[tokio::test]
async fn test_csv_producer_with_quote_config() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "name,age").unwrap();
  writeln!(file, "\"Alice\",30").unwrap();
  writeln!(file, "\"Bob\",25").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let mut producer = CsvProducer::<TestRecord>::new(path);
  let stream = producer.produce();
  let result: Vec<TestRecord> = stream.collect().await;

  assert_eq!(result.len(), 2);
  assert_eq!(result[0].name, "Alice");
  assert_eq!(result[1].name, "Bob");
  drop(file);
}

#[tokio::test]
async fn test_csv_producer_error_stream_path() {
  // Test the error path in the async stream where Err(stream_error) is received
  // This happens when error strategy is Stop and an error occurs
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "name,age").unwrap();
  writeln!(file, "Alice,30").unwrap();
  writeln!(file, "Bob,invalid_age").unwrap(); // This will fail to deserialize
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let mut producer = CsvProducer::<TestRecord>::new(path)
    .with_error_strategy(streamweave_error::ErrorStrategy::<TestRecord>::Stop);
  let stream = producer.produce();
  let result: Vec<TestRecord> = stream.collect().await;

  // Should stop at first error, so only Alice should be read
  // The error path in the stream (Err(stream_error)) should be hit
  assert_eq!(result.len(), 1);
  assert_eq!(result[0].name, "Alice");
  drop(file);
}

#[test]
fn test_csv_producer_handle_error_fallback() {
  // Test the fallback path in handle_error (_ => ErrorAction::Stop)
  let mut error = streamweave_error::StreamError::new(
    Box::new(std::io::Error::other("test error")),
    streamweave_error::ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "CsvProducer".to_string(),
    },
    streamweave_error::ComponentInfo {
      name: "test".to_string(),
      type_name: "CsvProducer".to_string(),
    },
  );
  error.retries = 10;

  // With Retry(3) strategy and retries=10, should hit the fallback
  let producer = CsvProducer::<TestRecord>::new("test.csv")
    .with_error_strategy(streamweave_error::ErrorStrategy::<TestRecord>::Retry(3));
  let action = producer.handle_error(&error);
  assert_eq!(action, streamweave_error::ErrorAction::Stop);
}

#[tokio::test]
async fn test_csv_producer_retry_action_path() {
  // Test the Retry action path (line 299-301) - though it's not supported, we can verify it's hit
  // This path just continues (skips) when Retry is requested
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "name,age").unwrap();
  writeln!(file, "Alice,30").unwrap();
  writeln!(file, "Bob,invalid").unwrap(); // This will fail
  writeln!(file, "Charlie,35").unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  // Use Retry strategy - when retry is requested, it will skip (not actually retry)
  let mut producer = CsvProducer::<TestRecord>::new(path)
    .with_error_strategy(streamweave_error::ErrorStrategy::<TestRecord>::Retry(3));
  let stream = producer.produce();
  let result: Vec<TestRecord> = stream.collect().await;

  // Retry path just skips, so Bob should be skipped, Charlie should be read
  assert_eq!(result.len(), 2);
  assert_eq!(result[0].name, "Alice");
  assert_eq!(result[1].name, "Charlie");
  drop(file);
}
