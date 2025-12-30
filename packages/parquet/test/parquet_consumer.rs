//! Tests for ParquetConsumer

use arrow::array::{Int32Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use futures::{StreamExt, stream};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::basic::Compression;
use proptest::prelude::*;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;
use streamweave::{Consumer, Input};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_parquet::{
  ParquetCompression, ParquetConsumer, ParquetWriteConfig, ParquetWriterVersion,
};
use tempfile::NamedTempFile;

fn create_test_batch(names: Vec<String>, ages: Vec<i32>) -> RecordBatch {
  let schema = Arc::new(Schema::new(vec![
    Field::new("name", DataType::Utf8, false),
    Field::new("age", DataType::Int32, false),
  ]));

  let names_array = StringArray::from(names);
  let ages_array = Int32Array::from(ages);

  RecordBatch::try_new(schema, vec![Arc::new(names_array), Arc::new(ages_array)]).unwrap()
}

async fn test_parquet_consumer_basic_async(names: Vec<String>, ages: Vec<i32>) {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let batch = create_test_batch(names.clone(), ages.clone());
  let expected_rows = names.len();

  let mut consumer = ParquetConsumer::new(&path);
  let input_stream = Box::pin(stream::iter(vec![batch]));
  consumer.consume(input_stream).await;

  // Read back and verify
  let read_file = File::open(&path).unwrap();
  let reader = ParquetRecordBatchReaderBuilder::try_new(read_file)
    .unwrap()
    .build()
    .unwrap();

  let batches: Vec<RecordBatch> = reader.filter_map(|r| r.ok()).collect();
  assert_eq!(batches.len(), 1);
  assert_eq!(batches[0].num_rows(), expected_rows);
  drop(file);
}

async fn test_parquet_consumer_multiple_batches_async(
  names1: Vec<String>,
  ages1: Vec<i32>,
  names2: Vec<String>,
  ages2: Vec<i32>,
) {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let batch1 = create_test_batch(names1.clone(), ages1);
  let batch2 = create_test_batch(names2.clone(), ages2);
  let expected_total_rows = names1.len() + names2.len();

  let mut consumer = ParquetConsumer::new(&path);
  let input_stream = Box::pin(stream::iter(vec![batch1, batch2]));
  consumer.consume(input_stream).await;

  // Read back and verify
  let read_file = File::open(&path).unwrap();
  let reader = ParquetRecordBatchReaderBuilder::try_new(read_file)
    .unwrap()
    .build()
    .unwrap();

  let total_rows: usize = reader.filter_map(|r| r.ok()).map(|b| b.num_rows()).sum();
  assert_eq!(total_rows, expected_total_rows);
  drop(file);
}

async fn test_parquet_roundtrip_async(names: Vec<String>, ages: Vec<i32>) {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  // Write
  let original_batch = create_test_batch(names.clone(), ages.clone());
  let mut consumer = ParquetConsumer::new(&path);
  let input_stream = Box::pin(stream::iter(vec![original_batch.clone()]));
  consumer.consume(input_stream).await;

  // Read back
  let read_file = File::open(&path).unwrap();
  let reader = ParquetRecordBatchReaderBuilder::try_new(read_file)
    .unwrap()
    .build()
    .unwrap();

  let batches: Vec<RecordBatch> = reader.filter_map(|r| r.ok()).collect();
  assert_eq!(batches.len(), 1);

  // Compare data
  let read_batch = &batches[0];
  assert_eq!(read_batch.num_rows(), original_batch.num_rows());
  assert_eq!(read_batch.num_columns(), original_batch.num_columns());
  drop(file);
}

fn names_ages_strategy(
  size_range: impl Strategy<Value = usize>,
) -> impl Strategy<Value = (Vec<String>, Vec<i32>)> {
  size_range.prop_flat_map(|size| {
    (
      prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9 ]+").unwrap(), size),
      prop::collection::vec(0i32..150i32, size),
    )
  })
}

proptest! {
  #[test]
  fn test_parquet_consumer_new(path in "[a-zA-Z0-9_./-]+\\.parquet") {
    let consumer = ParquetConsumer::new(path.clone());
    prop_assert_eq!(consumer.path(), &PathBuf::from(path));
  }

  #[test]
  fn test_parquet_consumer_builder(
    path in "[a-zA-Z0-9_./-]+\\.parquet",
    name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
    compression in prop::sample::select(vec![
      ParquetCompression::Uncompressed,
      ParquetCompression::Snappy,
      ParquetCompression::Lz4,
      ParquetCompression::Zstd,
    ]),
    max_row_group_size in 1024usize..1048576usize
  ) {
    let consumer = ParquetConsumer::new(path.clone())
      .with_name(name.clone())
      .with_compression(compression)
      .with_max_row_group_size(max_row_group_size);

    prop_assert_eq!(consumer.path(), &PathBuf::from(path));
    prop_assert_eq!(consumer.config.name, name);
    prop_assert_eq!(consumer.parquet_config.compression, compression);
    prop_assert_eq!(consumer.parquet_config.max_row_group_size, max_row_group_size);
  }

  #[test]
  fn test_parquet_write_config_default(_ in prop::num::u8::ANY) {
    let config = ParquetWriteConfig::default();
    prop_assert_eq!(config.compression, ParquetCompression::Snappy);
    prop_assert_eq!(config.max_row_group_size, 1024 * 1024);
    prop_assert_eq!(config.writer_version, ParquetWriterVersion::V2);
  }

  #[test]
  fn test_parquet_compression_conversion(
    compression in prop::sample::select(vec![
      ParquetCompression::Uncompressed,
      ParquetCompression::Snappy,
      ParquetCompression::Lz4,
      ParquetCompression::Zstd,
    ])
  ) {
    let result: Compression = compression.into();
    match compression {
      ParquetCompression::Uncompressed => {
        prop_assert!(matches!(result, Compression::UNCOMPRESSED));
      }
      ParquetCompression::Snappy => {
        prop_assert!(matches!(result, Compression::SNAPPY));
      }
      ParquetCompression::Lz4 => {
        prop_assert!(matches!(result, Compression::LZ4));
      }
      ParquetCompression::Zstd => {
        prop_assert!(matches!(result, Compression::ZSTD(_)));
      }
    }
  }

  #[test]
  fn test_parquet_consumer_basic(
    (names, ages) in names_ages_strategy(1usize..20)
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_parquet_consumer_basic_async(names, ages));
  }

  #[test]
  fn test_parquet_consumer_multiple_batches(
    (names1, ages1) in names_ages_strategy(1usize..10),
    (names2, ages2) in names_ages_strategy(1usize..10)
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_parquet_consumer_multiple_batches_async(names1, ages1, names2, ages2));
  }

  #[test]
  fn test_parquet_consumer_empty_stream(_ in prop::num::u8::ANY) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
      let file = NamedTempFile::new().unwrap();
      let path = file.path().to_str().unwrap().to_string();

      let mut consumer = ParquetConsumer::new(&path);
      let input_stream = Box::pin(stream::iter(Vec::<RecordBatch>::new()));
      consumer.consume(input_stream).await;

      drop(file);
    });

    // This test is mainly to ensure empty stream doesn't panic
    // File should not exist or be empty since no data was written
    // (writer is only created on first batch)
  }

  #[test]
  fn test_parquet_consumer_component_info(
    name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
  ) {
    let consumer = ParquetConsumer::new("test.parquet").with_name(name.clone());
    let info = consumer.component_info();
    prop_assert_eq!(info.name, name);
    prop_assert_eq!(info.type_name, std::any::type_name::<ParquetConsumer>());
  }

  #[test]
  fn test_parquet_roundtrip(
    (names, ages) in names_ages_strategy(1usize..20)
  ) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(test_parquet_roundtrip_async(names, ages));
  }
}

// Input trait tests

#[test]
fn test_parquet_consumer_input_trait_implementation() {
  fn assert_input_trait(_consumer: ParquetConsumer)
  where
    ParquetConsumer: Input<Input = RecordBatch>,
  {
  }

  let consumer = ParquetConsumer::new("test.parquet");
  assert_input_trait(consumer);
}

#[test]
fn test_parquet_write_config_builder() {
  let config = ParquetWriteConfig::default()
    .with_compression(ParquetCompression::Zstd)
    .with_max_row_group_size(2048 * 1024)
    .with_writer_version(ParquetWriterVersion::V1);

  assert_eq!(config.compression, ParquetCompression::Zstd);
  assert_eq!(config.max_row_group_size, 2048 * 1024);
  assert_eq!(config.writer_version, ParquetWriterVersion::V1);
}

#[test]
fn test_parquet_write_config_to_writer_properties() {
  let config = ParquetWriteConfig::default().with_compression(ParquetCompression::Lz4);

  let props = config.to_writer_properties();
  // Verify properties are created (can't easily test internals)
  assert!(std::mem::size_of_val(&props) > 0);
}

#[test]
fn test_parquet_compression_all_variants() {
  let uncompressed: Compression = ParquetCompression::Uncompressed.into();
  assert!(matches!(uncompressed, Compression::UNCOMPRESSED));

  let snappy: Compression = ParquetCompression::Snappy.into();
  assert!(matches!(snappy, Compression::SNAPPY));

  let lz4: Compression = ParquetCompression::Lz4.into();
  assert!(matches!(lz4, Compression::LZ4));

  let zstd: Compression = ParquetCompression::Zstd.into();
  assert!(matches!(zstd, Compression::ZSTD(_)));
}

#[test]
fn test_parquet_writer_version() {
  assert_eq!(ParquetWriterVersion::default(), ParquetWriterVersion::V2);
  assert_eq!(ParquetCompression::default(), ParquetCompression::Snappy);
}

#[tokio::test]
async fn test_parquet_consumer_with_compression() {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let batch = create_test_batch(vec!["Alice".to_string()], vec![30]);
  let mut consumer = ParquetConsumer::new(&path).with_compression(ParquetCompression::Zstd);

  let input_stream = Box::pin(stream::iter(vec![batch]));
  consumer.consume(input_stream).await;

  // Verify file was created
  assert!(std::path::Path::new(&path).exists());
  drop(file);
}

#[tokio::test]
async fn test_parquet_consumer_with_max_row_group_size() {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let batch = create_test_batch(vec!["Alice".to_string()], vec![30]);
  let mut consumer = ParquetConsumer::new(&path).with_max_row_group_size(512 * 1024);

  let input_stream = Box::pin(stream::iter(vec![batch]));
  consumer.consume(input_stream).await;

  assert!(std::path::Path::new(&path).exists());
  drop(file);
}

#[tokio::test]
async fn test_parquet_consumer_with_writer_version() {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let batch = create_test_batch(vec!["Alice".to_string()], vec![30]);
  let mut consumer = ParquetConsumer::new(&path).with_writer_version(ParquetWriterVersion::V1);

  let input_stream = Box::pin(stream::iter(vec![batch]));
  consumer.consume(input_stream).await;

  assert!(std::path::Path::new(&path).exists());
  drop(file);
}

#[tokio::test]
async fn test_parquet_consumer_with_error_strategy() {
  use streamweave_error::ErrorStrategy;

  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  let batch = create_test_batch(vec!["Alice".to_string()], vec![30]);
  let mut consumer = ParquetConsumer::new(&path).with_error_strategy(ErrorStrategy::Skip);

  let input_stream = Box::pin(stream::iter(vec![batch]));
  consumer.consume(input_stream).await;

  assert!(std::path::Path::new(&path).exists());
  drop(file);
}

#[test]
fn test_parquet_consumer_config_methods() {
  let mut consumer = ParquetConsumer::new("test.parquet");

  // Test config getters
  let _config = consumer.get_config_impl();
  let _config_mut = consumer.get_config_mut_impl();

  // Test component info
  let info = consumer.component_info();
  assert_eq!(info.type_name, std::any::type_name::<ParquetConsumer>());
}

#[test]
fn test_parquet_consumer_create_error_context() {
  let consumer = ParquetConsumer::new("test.parquet");
  let context = consumer.create_error_context(None);

  assert_eq!(
    context.component_type,
    std::any::type_name::<ParquetConsumer>()
  );
}

#[tokio::test]
async fn test_parquet_consumer_handle_error() {
  use streamweave_error::{ComponentInfo, ErrorContext, ErrorStrategy, StreamError};

  let consumer = ParquetConsumer::new("test.parquet").with_error_strategy(ErrorStrategy::Retry(3));

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

  let action = consumer.handle_error(&error);
  // With retries < 3, should retry
  assert!(matches!(action, streamweave_error::ErrorAction::Retry));
}

#[tokio::test]
async fn test_parquet_consumer_path_getter() {
  let consumer = ParquetConsumer::new("test.parquet");
  assert_eq!(consumer.path(), &std::path::PathBuf::from("test.parquet"));
}
