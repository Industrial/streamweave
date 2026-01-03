//! Tests for JsonlProducer

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::io::Write;
use streamweave::Producer;
use streamweave::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_jsonl::JsonlProducer;
use tempfile::NamedTempFile;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TestData {
  id: u32,
  name: String,
}

#[tokio::test]
async fn test_jsonl_producer_basic() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, r#"{"id": 1, "name": "Alice"}"#).unwrap();
  writeln!(file, r#"{"id": 2, "name": "Bob"}"#).unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let mut producer = JsonlProducer::<TestData>::new(path);
  let stream = producer.produce();
  let result: Vec<TestData> = stream.collect().await;

  assert_eq!(
    result,
    vec![
      TestData {
        id: 1,
        name: "Alice".to_string()
      },
      TestData {
        id: 2,
        name: "Bob".to_string()
      },
    ]
  );
  drop(file);
}

#[tokio::test]
async fn test_jsonl_producer_empty_file() {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let mut producer = JsonlProducer::<TestData>::new(path);
  let stream = producer.produce();
  let result: Vec<TestData> = stream.collect().await;

  assert!(result.is_empty());
  drop(file);
}

#[tokio::test]
async fn test_jsonl_producer_nonexistent_file() {
  let mut producer = JsonlProducer::<TestData>::new("nonexistent.jsonl".to_string());
  let stream = producer.produce();
  let result: Vec<TestData> = stream.collect().await;

  assert!(result.is_empty());
}

#[tokio::test]
async fn test_jsonl_producer_malformed_json_skip() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, r#"{"id": 1, "name": "Alice"}"#).unwrap();
  writeln!(file, r#"malformed json"#).unwrap();
  writeln!(file, r#"{"id": 2, "name": "Bob"}"#).unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  // Default error strategy is Stop, so only first item will be produced
  let mut producer = JsonlProducer::<TestData>::new(path).with_error_strategy(ErrorStrategy::Skip);
  let stream = producer.produce();
  let result: Vec<TestData> = stream.collect().await;

  // Both valid items should be produced
  assert_eq!(
    result,
    vec![
      TestData {
        id: 1,
        name: "Alice".to_string()
      },
      TestData {
        id: 2,
        name: "Bob".to_string()
      },
    ]
  );
  drop(file);
}

#[tokio::test]
async fn test_jsonl_producer_error_strategy_stop() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, r#"{"id": 1, "name": "Alice"}"#).unwrap();
  writeln!(file, r#"malformed json"#).unwrap(); // This should stop the stream
  writeln!(file, r#"{"id": 2, "name": "Bob"}"#).unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let mut producer = JsonlProducer::<TestData>::new(path).with_error_strategy(ErrorStrategy::Stop);
  let stream = producer.produce();
  let result: Vec<TestData> = stream.collect().await;

  // Only the first valid item should be produced before stopping
  assert_eq!(
    result,
    vec![TestData {
      id: 1,
      name: "Alice".to_string()
    }]
  );
  drop(file);
}

#[tokio::test]
async fn test_jsonl_producer_component_info() {
  let producer =
    JsonlProducer::<TestData>::new("path.jsonl").with_name("my_jsonl_producer".to_string());
  let info = producer.component_info();
  assert_eq!(info.name, "my_jsonl_producer");
  assert_eq!(
    info.type_name,
    std::any::type_name::<JsonlProducer<TestData>>()
  );
}

#[test]
fn test_jsonl_producer_new() {
  let producer = JsonlProducer::<TestData>::new("test.jsonl");
  assert_eq!(producer.path(), "test.jsonl");
}

#[test]
fn test_jsonl_producer_builder() {
  let producer = JsonlProducer::<TestData>::new("test.jsonl")
    .with_name("test_producer".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  assert_eq!(producer.path(), "test.jsonl");
  assert_eq!(producer.config.name, Some("test_producer".to_string()));
  assert!(matches!(
    producer.config.error_strategy,
    ErrorStrategy::Skip
  ));
}

#[test]
fn test_jsonl_producer_clone() {
  let producer =
    JsonlProducer::<TestData>::new("test.jsonl").with_name("test_producer".to_string());

  let cloned = producer.clone();
  assert_eq!(cloned.path(), producer.path());
  assert_eq!(cloned.config.name, producer.config.name);
}

#[tokio::test]
async fn test_jsonl_producer_error_strategy_retry() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, r#"{"id": 1, "name": "Alice"}"#).unwrap();
  writeln!(file, r#"malformed json"#).unwrap();
  writeln!(file, r#"{"id": 2, "name": "Bob"}"#).unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  // Retry strategy should skip (as retry is not fully supported for deserialization)
  let mut producer =
    JsonlProducer::<TestData>::new(path).with_error_strategy(ErrorStrategy::Retry(3));
  let stream = producer.produce();
  let result: Vec<TestData> = stream.collect().await;

  // Should get first item, then skip malformed, then get second item
  assert_eq!(
    result,
    vec![
      TestData {
        id: 1,
        name: "Alice".to_string()
      },
      TestData {
        id: 2,
        name: "Bob".to_string()
      },
    ]
  );
  drop(file);
}

#[tokio::test]
async fn test_jsonl_producer_error_strategy_custom() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, r#"{"id": 1, "name": "Alice"}"#).unwrap();
  writeln!(file, r#"malformed json"#).unwrap();
  writeln!(file, r#"{"id": 2, "name": "Bob"}"#).unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  // Custom handler that always skips
  let custom_handler = |_error: &StreamError<TestData>| ErrorAction::Skip;
  let mut producer = JsonlProducer::<TestData>::new(path)
    .with_error_strategy(ErrorStrategy::new_custom(custom_handler));
  let stream = producer.produce();
  let result: Vec<TestData> = stream.collect().await;

  assert_eq!(
    result,
    vec![
      TestData {
        id: 1,
        name: "Alice".to_string()
      },
      TestData {
        id: 2,
        name: "Bob".to_string()
      },
    ]
  );
  drop(file);
}

#[tokio::test]
async fn test_jsonl_producer_error_strategy_custom_stop() {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, r#"{"id": 1, "name": "Alice"}"#).unwrap();
  writeln!(file, r#"malformed json"#).unwrap();
  writeln!(file, r#"{"id": 2, "name": "Bob"}"#).unwrap();
  file.flush().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  // Custom handler that always stops
  let custom_handler = |_error: &StreamError<TestData>| ErrorAction::Stop;
  let mut producer = JsonlProducer::<TestData>::new(path)
    .with_error_strategy(ErrorStrategy::new_custom(custom_handler));
  let stream = producer.produce();
  let result: Vec<TestData> = stream.collect().await;

  // Should stop at first error
  assert_eq!(
    result,
    vec![TestData {
      id: 1,
      name: "Alice".to_string()
    }]
  );
  drop(file);
}

#[test]
fn test_jsonl_producer_config_methods() {
  let mut producer = JsonlProducer::<TestData>::new("test.jsonl");
  let config = producer.config().clone();
  assert_eq!(config.name, None);

  let config_mut = producer.config_mut();
  config_mut.name = Some("test".to_string());
  assert_eq!(producer.config().name, Some("test".to_string()));

  let new_config = streamweave::ProducerConfig::default();
  producer.set_config(new_config.clone());
  assert_eq!(producer.config().name, new_config.name);
}

#[test]
fn test_jsonl_producer_create_error_context() {
  let producer =
    JsonlProducer::<TestData>::new("test.jsonl").with_name("test_producer".to_string());
  let ctx = producer.create_error_context(Some(TestData {
    id: 42,
    name: "test".to_string(),
  }));
  assert_eq!(ctx.component_name, "test_producer");
  assert_eq!(
    ctx.item,
    Some(TestData {
      id: 42,
      name: "test".to_string()
    })
  );

  let ctx_no_item = producer.create_error_context(None);
  assert_eq!(ctx_no_item.component_name, "test_producer");
  assert_eq!(ctx_no_item.item, None);
}

#[test]
fn test_jsonl_producer_handle_error() {
  let producer =
    JsonlProducer::<TestData>::new("test.jsonl").with_error_strategy(ErrorStrategy::Skip);
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
  assert_eq!(producer.handle_error(&error), ErrorAction::Skip);

  let producer_stop =
    JsonlProducer::<TestData>::new("test.jsonl").with_error_strategy(ErrorStrategy::Stop);
  assert_eq!(producer_stop.handle_error(&error), ErrorAction::Stop);

  let producer_retry =
    JsonlProducer::<TestData>::new("test.jsonl").with_error_strategy(ErrorStrategy::Retry(3));
  let mut error_retry = error.clone();
  error_retry.retries = 0;
  assert_eq!(
    producer_retry.handle_error(&error_retry),
    ErrorAction::Retry
  );

  let mut error_retry_max = error.clone();
  error_retry_max.retries = 3;
  assert_eq!(
    producer_retry.handle_error(&error_retry_max),
    ErrorAction::Stop
  );
}

#[test]
fn test_jsonl_producer_default_name() {
  let producer = JsonlProducer::<TestData>::new("test.jsonl");
  let info = producer.component_info();
  assert_eq!(info.name, "jsonl_producer");
}
