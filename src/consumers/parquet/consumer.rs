use super::parquet_consumer::ParquetConsumer;
use crate::consumer::{Consumer, ConsumerConfig};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;
use futures::StreamExt;
use parquet::arrow::ArrowWriter;
use std::fs::File;
use tracing::{error, warn};

#[async_trait]
impl Consumer for ParquetConsumer {
  type InputPorts = (arrow::record_batch::RecordBatch,);

  /// Consumes a stream of Arrow RecordBatches and writes them to a Parquet file.
  ///
  /// # Error Handling
  ///
  /// - If the file cannot be created, an error is logged and no data is written.
  /// - If a batch cannot be written, the error strategy determines the action.
  async fn consume(&mut self, input: Self::InputStream) {
    let path = self.path.clone();
    let component_name = if self.config.name.is_empty() {
      "parquet_consumer".to_string()
    } else {
      self.config.name.clone()
    };
    let error_strategy = self.config.error_strategy.clone();
    let parquet_config = self.parquet_config.clone();

    let mut input = std::pin::pin!(input);
    let mut writer: Option<ArrowWriter<File>> = None;

    while let Some(batch) = input.next().await {
      // Initialize writer on first batch
      if writer.is_none() {
        let file = match File::create(&path) {
          Ok(f) => f,
          Err(e) => {
            error!(
              component = %component_name,
              path = %path.display(),
              error = %e,
              "Failed to create Parquet file for writing"
            );
            return;
          }
        };

        let props = parquet_config.to_writer_properties();

        match ArrowWriter::try_new(file, batch.schema(), Some(props)) {
          Ok(w) => writer = Some(w),
          Err(e) => {
            error!(
              component = %component_name,
              path = %path.display(),
              error = %e,
              "Failed to create Parquet writer"
            );
            return;
          }
        }
      }

      // Write the batch
      if let Some(ref mut w) = writer
        && let Err(e) = w.write(&batch)
      {
        let stream_error = StreamError::new(
          Box::new(e),
          ErrorContext {
            timestamp: chrono::Utc::now(),
            item: Some(batch.clone()),
            component_name: component_name.clone(),
            component_type: std::any::type_name::<Self>().to_string(),
          },
          ComponentInfo {
            name: component_name.clone(),
            type_name: std::any::type_name::<Self>().to_string(),
          },
        );

        match handle_error_strategy(&error_strategy, &stream_error) {
          ErrorAction::Stop => {
            error!(
              component = %component_name,
              error = %stream_error,
              "Stopping due to write error"
            );
            break;
          }
          ErrorAction::Skip => {
            warn!(
              component = %component_name,
              error = %stream_error,
              "Skipping batch due to write error"
            );
            continue;
          }
          ErrorAction::Retry => {
            warn!(
              component = %component_name,
              error = %stream_error,
              "Retry not supported for Parquet write errors, skipping"
            );
            continue;
          }
        }
      }
    }

    // Close the writer
    if let Some(w) = writer
      && let Err(e) = w.close()
    {
      error!(
        component = %component_name,
        error = %e,
        "Failed to close Parquet writer"
      );
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<RecordBatch>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<RecordBatch> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<RecordBatch> {
    &mut self.config
  }
}

/// Helper function to handle error strategy
fn handle_error_strategy(
  strategy: &ErrorStrategy<RecordBatch>,
  error: &StreamError<RecordBatch>,
) -> ErrorAction {
  match strategy {
    ErrorStrategy::Stop => ErrorAction::Stop,
    ErrorStrategy::Skip => ErrorAction::Skip,
    ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
    ErrorStrategy::Custom(handler) => handler(error),
    _ => ErrorAction::Stop,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use arrow::array::{Int32Array, StringArray};
  use arrow::datatypes::{DataType, Field, Schema};
  use futures::stream;
  use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
  use proptest::prelude::*;
  use proptest::proptest;
  use std::sync::Arc;
  use tempfile::NamedTempFile;
  use tokio::runtime::Runtime;

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
    fn test_parquet_consumer_basic(
      (names, ages) in names_ages_strategy(1usize..20)
    ) {
      let rt = Runtime::new().unwrap();
      rt.block_on(test_parquet_consumer_basic_async(names, ages));
    }

    #[test]
    fn test_parquet_consumer_multiple_batches(
      (names1, ages1) in names_ages_strategy(1usize..10),
      (names2, ages2) in names_ages_strategy(1usize..10)
    ) {
      let rt = Runtime::new().unwrap();
      rt.block_on(test_parquet_consumer_multiple_batches_async(names1, ages1, names2, ages2));
    }

    #[test]
    fn test_parquet_consumer_empty_stream(_ in prop::num::u8::ANY) {
      let rt = Runtime::new().unwrap();
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
      let rt = Runtime::new().unwrap();
      rt.block_on(test_parquet_roundtrip_async(names, ages));
    }
  }
}
