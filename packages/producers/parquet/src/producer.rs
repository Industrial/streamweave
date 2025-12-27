use super::parquet_producer::ParquetProducer;
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::fs::File;
use streamweave_core::{Producer, ProducerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tracing::error;

#[async_trait]
impl Producer for ParquetProducer {
  type OutputPorts = (arrow::record_batch::RecordBatch,);

  /// Produces a stream of Arrow RecordBatches from a Parquet file.
  ///
  /// # Error Handling
  ///
  /// - If the file cannot be opened or is not valid Parquet, an error is logged
  ///   and an empty stream is returned.
  /// - If a batch cannot be read, the error strategy determines the action.
  fn produce(&mut self) -> Self::OutputStream {
    let path = self.path.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "parquet_producer".to_string());
    let error_strategy = self.config.error_strategy.clone();
    let parquet_config = self.parquet_config.clone();

    // Parquet reading is synchronous, so we use blocking_task wrapper
    Box::pin(async_stream::stream! {
      let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<RecordBatch, StreamError<RecordBatch>>>(100);

      let component_name_clone = component_name.clone();
      let error_strategy_clone = error_strategy.clone();

      let handle = tokio::task::spawn_blocking(move || {
        let file = match File::open(&path) {
          Ok(f) => f,
          Err(e) => {
            error!(
              component = %component_name_clone,
              path = %path,
              error = %e,
              "Failed to open Parquet file"
            );
            return;
          }
        };

        let builder = match ParquetRecordBatchReaderBuilder::try_new(file) {
          Ok(b) => b,
          Err(e) => {
            error!(
              component = %component_name_clone,
              path = %path,
              error = %e,
              "Failed to create Parquet reader"
            );
            return;
          }
        };

        // Apply configuration
        let mut builder = builder.with_batch_size(parquet_config.batch_size);

        if let Some(ref projection) = parquet_config.projection {
          // Create projection mask from indices
          let projection_mask = parquet::arrow::ProjectionMask::roots(
            builder.parquet_schema(),
            projection.iter().copied()
          );
          builder = builder.with_projection(projection_mask);
        }

        if let Some(ref row_groups) = parquet_config.row_groups {
          builder = builder.with_row_groups(row_groups.clone());
        }

        let reader = match builder.build() {
          Ok(r) => r,
          Err(e) => {
            error!(
              component = %component_name_clone,
              path = %path,
              error = %e,
              "Failed to build Parquet reader"
            );
            return;
          }
        };

        for result in reader {
          match result {
            Ok(batch) => {
              if tx.blocking_send(Ok(batch)).is_err() {
                break;
              }
            }
            Err(e) => {
              let stream_error = StreamError::new(
                Box::new(e),
                ErrorContext {
                  timestamp: chrono::Utc::now(),
                  item: None,
                  component_name: component_name_clone.clone(),
                  component_type: std::any::type_name::<ParquetProducer>().to_string(),
                },
                ComponentInfo {
                  name: component_name_clone.clone(),
                  type_name: std::any::type_name::<ParquetProducer>().to_string(),
                },
              );

              match handle_error_strategy(&error_strategy_clone, &stream_error) {
                ErrorAction::Stop => {
                  let _ = tx.blocking_send(Err(stream_error));
                  break;
                }
                ErrorAction::Skip => {
                  // Continue to next batch
                }
                ErrorAction::Retry => {
                  // Retry not supported for Parquet batch reading, skip
                }
              }
            }
          }
        }
      });

      while let Some(result) = rx.recv().await {
        match result {
          Ok(batch) => yield batch,
          Err(stream_error) => {
            error!(
              component = %component_name,
              error = %stream_error,
              "Error reading Parquet batch"
            );
            break;
          }
        }
      }

      // Wait for the blocking task to finish
      let _ = handle.await;
    })
  }

  fn set_config_impl(&mut self, config: ProducerConfig<RecordBatch>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<RecordBatch> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<RecordBatch> {
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
  use futures::StreamExt;
  use parquet::arrow::ArrowWriter;
  use std::sync::Arc;
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

    let batch =
      RecordBatch::try_new(schema.clone(), vec![Arc::new(names), Arc::new(ages)]).unwrap();

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
    let producer =
      ParquetProducer::new("path.parquet").with_name("my_parquet_producer".to_string());
    let info = producer.component_info();
    assert_eq!(info.name, "my_parquet_producer");
    assert_eq!(info.type_name, std::any::type_name::<ParquetProducer>());
  }
}
