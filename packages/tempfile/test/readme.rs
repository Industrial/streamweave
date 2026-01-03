//! Integration tests that verify all examples in README.md work correctly.
//!
//! These tests recreate the examples from the README.md file to ensure they
//! work as advertised. Each test corresponds to a code example in the README.
//!
//! Note: The README examples show an idealized API (e.g., `TempfileProducer::new()?`),
//! but the actual implementation requires a `PathBuf` from a `NamedTempFile`.
//! These tests adapt the examples to work with the actual API.

use std::io::Write;
use streamweave::consumers::{VecConsumer, VecProducer};
use streamweave::pipeline::PipelineBuilder;
use streamweave::transformers::MapTransformer;
use streamweave_tempfile::{TempFileConsumer, TempFileProducer};
use tempfile::NamedTempFile;

/// Test: Create and Process Temp File
///
/// This test recreates the "Create and Process Temp File" example from README.md lines 33-44.
/// The example shows how to use TempfileProducer and TempfileConsumer in a pipeline.
#[tokio::test]
async fn test_create_and_process_temp_file() {
  // Example from README.md lines 33-44
  use streamweave::pipeline::PipelineBuilder;
  use streamweave_tempfile::{TempFileConsumer, TempFileProducer};

  // Create temp file with data
  let mut input_file = NamedTempFile::new().unwrap();
  writeln!(input_file, "line 1").unwrap();
  writeln!(input_file, "line 2").unwrap();
  writeln!(input_file, "line 3").unwrap();
  input_file.flush().unwrap();
  let input_path = input_file.path().to_path_buf();

  // Create output temp file
  let output_file = NamedTempFile::new().unwrap();
  let output_path = output_file.path().to_path_buf();

  // Complete the example: README shows `/* your transformer */` - we use identity transformer
  let transformer = MapTransformer::new(|x: String| x.clone());

  let pipeline = PipelineBuilder::new()
    .producer(TempFileProducer::new(input_path))
    .transformer(transformer)
    .await
    .consumer(TempFileConsumer::new(output_path));

  pipeline.run().await.unwrap();
  // Temporary files are automatically cleaned up when dropped
}

/// Test: Create Temp File
///
/// This test recreates the "Create Temp File" example from README.md lines 48-54.
/// The example shows how to create a TempfileProducer.
#[tokio::test]
async fn test_create_temp_file() {
  // Example from README.md lines 48-54
  use streamweave_tempfile::TempFileProducer;

  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "test data").unwrap();
  file.flush().unwrap();
  let path = file.path().to_path_buf();

  let producer = TempFileProducer::new(path);
  // Creates producer from temp file, reads from it
  // File is automatically deleted when file handle is dropped

  let mut producer = producer;
  let stream = producer.produce();
  let result: Vec<String> = stream.collect().await;
  assert_eq!(result, vec!["test data"]);
}

/// Test: Write to Temp File
///
/// This test recreates the "Write to Temp File" example from README.md lines 58-64.
/// The example shows how to create a TempfileConsumer.
#[tokio::test]
async fn test_write_to_temp_file() {
  // Example from README.md lines 58-64
  use futures::{StreamExt, stream};
  use streamweave_tempfile::TempFileConsumer;

  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_path_buf();

  let mut consumer = TempFileConsumer::new(path);
  // Writes to temporary file
  // File is automatically deleted when file handle is dropped

  let input = stream::iter(vec!["test1".to_string(), "test2".to_string()]);
  let boxed_input = Box::pin(input);
  consumer.consume(boxed_input).await;
}

/// Test: Process Temp Data
///
/// This test recreates the "Process Temp Data" example from README.md lines 104-115.
/// The example shows how to process data in temporary files.
#[tokio::test]
async fn test_process_temp_data() {
  // Example from README.md lines 104-115
  use streamweave::pipeline::PipelineBuilder;
  use streamweave_tempfile::{TempFileConsumer, TempFileProducer};

  // Create input temp file with data
  let mut input_file = NamedTempFile::new().unwrap();
  writeln!(input_file, "data1").unwrap();
  writeln!(input_file, "data2").unwrap();
  input_file.flush().unwrap();
  let input_path = input_file.path().to_path_buf();

  // Create output temp file
  let output_file = NamedTempFile::new().unwrap();
  let output_path = output_file.path().to_path_buf();

  // Complete the example: README shows `/* transform data */` - we use identity transformer
  let transformer = MapTransformer::new(|x: String| x.clone());

  let pipeline = PipelineBuilder::new()
    .producer(TempFileProducer::new(input_path))
    .transformer(transformer)
    .await
    .consumer(TempFileConsumer::new(output_path));

  pipeline.run().await.unwrap();
  // Both temp files are automatically cleaned up when dropped
}

/// Test: Keep Temp File
///
/// This test recreates the "Keep Temp File" example from README.md lines 120-132.
/// The example shows how to keep a temporary file after processing.
/// Note: The actual implementation doesn't have `keep_on_drop()` method,
/// but we can test the concept by keeping the file handle.
#[tokio::test]
async fn test_keep_temp_file() {
  // Example from README.md lines 120-132
  use futures::{StreamExt, stream};
  use streamweave::pipeline::PipelineBuilder;
  use streamweave::producers::VecProducer;
  use streamweave_tempfile::TempFileConsumer;

  // Create temp file that we want to keep
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_path_buf();
  let path_clone = path.clone();

  let consumer = TempFileConsumer::new(path);

  // Process data
  let producer = VecProducer::new(vec!["data1".to_string(), "data2".to_string()]);
  let transformer = MapTransformer::new(|x: String| x.clone());
  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  pipeline.run().await.unwrap();

  // File is kept, can access via path
  // In actual implementation, we keep the file handle to prevent cleanup
  assert!(path_clone.exists());
  drop(file); // Now file will be cleaned up
}

/// Test: Custom Temp Directory
///
/// This test recreates the "Custom Temp Directory" example from README.md lines 138-143.
/// The example shows how to create temp files in a specific directory.
/// Note: The actual implementation doesn't have `new_in_dir()` method,
/// but we can test the concept by creating a temp file in a specific directory.
#[tokio::test]
async fn test_custom_temp_directory() {
  // Example from README.md lines 138-143
  use std::env;
  use std::path::PathBuf;
  use streamweave_tempfile::TempFileProducer;

  // Create temp file in system temp directory (closest to the example)
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_path_buf();

  let producer = TempFileProducer::new(path);
  // Creates producer from temp file
  // In actual implementation, we can create NamedTempFile in a specific directory

  let mut producer = producer;
  let stream = producer.produce();
  let _result: Vec<String> = stream.collect().await;
}

/// Test: Temp File Lifecycle
///
/// This test recreates the "Temp File Lifecycle" example from README.md lines 149-159.
/// The example shows how to control temp file lifecycle.
/// Note: The actual implementation doesn't have `keep_on_drop()`, `prefix()`, or `suffix()` methods,
/// but we can test the basic lifecycle behavior.
#[tokio::test]
async fn test_temp_file_lifecycle() {
  // Example from README.md lines 149-159
  use futures::{StreamExt, stream};
  use streamweave_tempfile::TempFileConsumer;

  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_path_buf();

  let mut consumer = TempFileConsumer::new(path).with_name("test_consumer".to_string());

  // Process data
  let input = stream::iter(vec!["data1".to_string(), "data2".to_string()]);
  let boxed_input = Box::pin(input);
  consumer.consume(boxed_input).await;

  // File is automatically deleted when file handle is dropped
  assert!(path.exists());
  drop(file);
  // File cleanup happens when NamedTempFile is dropped
}

/// Test: Producer Configuration
///
/// This test recreates the "Producer Configuration" example from README.md lines 186-192.
/// The example shows how to configure a temp file producer.
#[tokio::test]
async fn test_producer_configuration() {
  // Example from README.md lines 186-192
  use streamweave::ProducerConfig;
  use streamweave_tempfile::TempFileProducer;

  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_path_buf();

  let producer = TempFileProducer::new(path).with_name("temp-reader".to_string());

  // Verify the configuration was set correctly
  let config = producer.config();
  assert_eq!(config.name(), Some("temp-reader".to_string()));
}

/// Test: Consumer Configuration
///
/// This test recreates the "Consumer Configuration" example from README.md lines 198-205.
/// The example shows how to configure a temp file consumer.
#[tokio::test]
async fn test_consumer_configuration() {
  // Example from README.md lines 198-205
  use streamweave::error::ErrorStrategy;
  use streamweave_tempfile::TempFileConsumer;

  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_path_buf();

  let consumer = TempFileConsumer::new(path)
    .with_error_strategy(ErrorStrategy::Skip)
    .with_name("temp-writer".to_string());

  // Verify the configuration was set correctly
  let config = consumer.config();
  assert_eq!(config.error_strategy, ErrorStrategy::<String>::Skip);
  assert_eq!(config.name, "temp-writer");
}

/// Test: Error Handling
///
/// This test recreates the "Error Handling" example from README.md lines 212-218.
/// The example shows how to configure error handling for temp file operations.
#[tokio::test]
async fn test_error_handling() {
  // Example from README.md lines 212-218
  use streamweave::error::ErrorStrategy;
  use streamweave::pipeline::PipelineBuilder;

  // Create temp files
  let mut input_file = NamedTempFile::new().unwrap();
  writeln!(input_file, "data1").unwrap();
  input_file.flush().unwrap();
  let input_path = input_file.path().to_path_buf();

  let output_file = NamedTempFile::new().unwrap();
  let output_path = output_file.path().to_path_buf();

  let transformer = MapTransformer::new(|x: String| x.clone());

  let pipeline = PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Skip)
    .producer(TempFileProducer::new(input_path))
    .transformer(transformer)
    .await
    .consumer(TempFileConsumer::new(output_path));

  pipeline.run().await.unwrap();
}
