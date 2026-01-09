//! Tests for ParquetProducer

use arrow::array::{Int32Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use futures::StreamExt;
use parquet::arrow::ArrowWriter;
use std::sync::Arc;
use streamweave::Producer;
use streamweave::producers::{ParquetProducer, ParquetReadConfig};
use tempfile::NamedTempFile;

fn create_test_parquet_file() -> NamedTempFile {
  let file = NamedTempFile::new().unwrap();

  // Create a simple schema and data
  let schema = Arc::new(Schema::new(vec![
    Field::new("name", DataType::Utf8, false),
    Field::new("age", DataType::Int32, false),
  ]));

  let names = StringArray::from(vec!["Alice", "Bob", "Charlie"]);
  let ages = Int32Array::from(vec![30, 25, 35]);

  let batch = RecordBatch::try_new(schema.clone(), vec![Arc::new(names), Arc::new(ages)]).unwrap();

  // Write to Parquet
  let writer_file = std::fs::File::create(file.path()).unwrap();
  let mut writer = ArrowWriter::try_new(writer_file, schema, None).unwrap();
  writer.write(&batch).unwrap();
  writer.close().unwrap();

  file
}

#[tokio::test]
async fn test_parquet_producer_basic() {
  let file = create_test_parquet_file();
  let path = file.path().to_str().unwrap().to_string();

  let mut producer = ParquetProducer::new(path);
  let stream = producer.produce();
  let batches: Vec<RecordBatch> = stream.collect().await;

  assert!(!batches.is_empty());
  let batch = &batches[0];
  assert_eq!(batch.num_rows(), 3);
  assert_eq!(batch.num_columns(), 2);
  drop(file);
}

#[tokio::test]
async fn test_parquet_producer_nonexistent_file() {
  let mut producer = ParquetProducer::new("nonexistent.parquet".to_string());
  let stream = producer.produce();
  let batches: Vec<RecordBatch> = stream.collect().await;

  assert!(batches.is_empty());
}

#[tokio::test]
async fn test_parquet_producer_with_batch_size() {
  let file = create_test_parquet_file();
  let path = file.path().to_str().unwrap().to_string();

  let mut producer = ParquetProducer::new(path).with_batch_size(1);
  let stream = producer.produce();
  let batches: Vec<RecordBatch> = stream.collect().await;

  // With batch size 1, we should get 3 batches
  assert_eq!(batches.len(), 3);
  for batch in batches {
    assert_eq!(batch.num_rows(), 1);
  }
  drop(file);
}

#[tokio::test]
async fn test_parquet_producer_component_info() {
  let producer = ParquetProducer::new("path.parquet").with_name("my_parquet_producer".to_string());
  let info = producer.component_info();
  assert_eq!(info.name, "my_parquet_producer");
  assert_eq!(info.type_name, std::any::type_name::<ParquetProducer>());
}

#[test]
fn test_parquet_producer_new() {
  let producer = ParquetProducer::new("test.parquet");
  assert_eq!(producer.path(), "test.parquet");
}

#[test]
fn test_parquet_producer_builder() {
  let producer = ParquetProducer::new("test.parquet")
    .with_name("test_producer".to_string())
    .with_batch_size(2048)
    .with_projection(vec![0, 1, 2]);

  assert_eq!(producer.path(), "test.parquet");
  assert_eq!(producer.config.name, Some("test_producer".to_string()));
  assert_eq!(producer.parquet_config.batch_size, 2048);
  assert_eq!(producer.parquet_config.projection, Some(vec![0, 1, 2]));
}

#[test]
fn test_parquet_read_config_default() {
  let config = ParquetReadConfig::default();
  assert_eq!(config.batch_size, 1024);
  assert_eq!(config.projection, None);
  assert_eq!(config.row_groups, None);
}

#[test]
fn test_parquet_read_config_builder() {
  let config = ParquetReadConfig::default()
    .with_batch_size(2048)
    .with_projection(vec![0, 2])
    .with_row_groups(vec![0, 1]);

  assert_eq!(config.batch_size, 2048);
  assert_eq!(config.projection, Some(vec![0, 2]));
  assert_eq!(config.row_groups, Some(vec![0, 1]));
}

#[test]
fn test_parquet_producer_clone() {
  let producer = ParquetProducer::new("test.parquet")
    .with_name("test".to_string())
    .with_batch_size(2048);

  let cloned = producer.clone();
  assert_eq!(cloned.path(), producer.path());
  assert_eq!(cloned.config.name, producer.config.name);
  assert_eq!(
    cloned.parquet_config.batch_size,
    producer.parquet_config.batch_size
  );
}

#[test]
fn test_parquet_producer_with_projection() {
  let file = create_test_parquet_file();
  let path = file.path().to_str().unwrap().to_string();

  let mut producer = ParquetProducer::new(path).with_projection(vec![0]); // Only read first column

  // This test verifies the projection is set
  assert_eq!(producer.parquet_config.projection, Some(vec![0]));
  drop(file);
}

#[test]
fn test_parquet_producer_with_row_groups() {
  let file = create_test_parquet_file();
  let path = file.path().to_str().unwrap().to_string();

  let mut producer = ParquetProducer::new(path).with_row_groups(vec![0]);

  // This test verifies row groups are set
  assert_eq!(producer.parquet_config.row_groups, Some(vec![0]));
  drop(file);
}

#[test]
fn test_parquet_producer_with_error_strategy() {
  use streamweave::error::ErrorStrategy;

  let producer = ParquetProducer::new("test.parquet").with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(
    producer.config.error_strategy,
    ErrorStrategy::Skip
  ));
}

#[test]
fn test_parquet_producer_config_methods() {
  let mut producer = ParquetProducer::new("test.parquet");

  // Test config getters
  let _config = producer.get_config_impl();
  let _config_mut = producer.get_config_mut_impl();

  // Test component info
  let info = producer.component_info();
  assert_eq!(info.type_name, std::any::type_name::<ParquetProducer>());
}

#[test]
fn test_parquet_producer_create_error_context() {
  let producer = ParquetProducer::new("test.parquet");
  let context = producer.create_error_context(None);

  assert_eq!(
    context.component_type,
    std::any::type_name::<ParquetProducer>()
  );
}

#[tokio::test]
async fn test_parquet_producer_handle_error() {
  use streamweave::error::{ComponentInfo, ErrorContext, ErrorStrategy, StreamError};

  let producer = ParquetProducer::new("test.parquet").with_error_strategy(ErrorStrategy::Skip);

  let error = StreamError::new(
    Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "test")),
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

  let action = producer.handle_error(&error);
  assert!(matches!(action, streamweave::error::ErrorAction::Skip));
}
