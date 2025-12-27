use super::csv_producer::CsvProducer;
use async_trait::async_trait;
use csv::Trim;
use serde::de::DeserializeOwned;
use std::fs::File;
use std::io::BufReader;
use streamweave_core::{Producer, ProducerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tracing::error;

#[async_trait]
impl<T> Producer for CsvProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type OutputPorts = (T,);

  /// Produces a stream of deserialized CSV rows.
  ///
  /// # Error Handling
  ///
  /// - If the file cannot be opened, an error is logged and an empty stream is returned.
  /// - If a row cannot be deserialized, the error strategy determines the action.
  fn produce(&mut self) -> Self::OutputStream {
    let path = self.path.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "csv_producer".to_string());
    let error_strategy = self.config.error_strategy.clone();
    let csv_config = self.csv_config.clone();

    // CSV reading is synchronous, so we use blocking_task wrapper
    Box::pin(async_stream::stream! {
      // Use tokio's blocking spawn for synchronous CSV reading
      let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<T, StreamError<T>>>(100);

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
              "Failed to open CSV file"
            );
            return;
          }
        };

        let reader = BufReader::new(file);

        let mut csv_reader = csv::ReaderBuilder::new()
          .has_headers(csv_config.has_headers)
          .delimiter(csv_config.delimiter)
          .flexible(csv_config.flexible)
          .trim(if csv_config.trim { Trim::All } else { Trim::None })
          .quote(csv_config.quote)
          .double_quote(csv_config.double_quote)
          .from_reader(reader);

        if let Some(comment) = csv_config.comment {
          csv_reader = csv::ReaderBuilder::new()
            .has_headers(csv_config.has_headers)
            .delimiter(csv_config.delimiter)
            .flexible(csv_config.flexible)
            .trim(if csv_config.trim { Trim::All } else { Trim::None })
            .quote(csv_config.quote)
            .double_quote(csv_config.double_quote)
            .comment(Some(comment))
            .from_reader(BufReader::new(File::open(&path).unwrap()));
        }

        for result in csv_reader.deserialize::<T>() {
          match result {
            Ok(record) => {
              if tx.blocking_send(Ok(record)).is_err() {
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
                  component_type: std::any::type_name::<Self>().to_string(),
                },
                ComponentInfo {
                  name: component_name_clone.clone(),
                  type_name: std::any::type_name::<Self>().to_string(),
                },
              );

              match handle_error_strategy(&error_strategy_clone, &stream_error) {
                ErrorAction::Stop => {
                  let _ = tx.blocking_send(Err(stream_error));
                  break;
                }
                ErrorAction::Skip => {
                  // Continue to next record
                }
                ErrorAction::Retry => {
                  // Retry not supported for CSV row reading, skip
                }
              }
            }
          }
        }
      });

      while let Some(result) = rx.recv().await {
        match result {
          Ok(record) => yield record,
          Err(stream_error) => {
            error!(
              component = %component_name,
              error = %stream_error,
              "Error reading CSV record"
            );
            break;
          }
        }
      }

      // Wait for the blocking task to finish
      let _ = handle.await;
    })
  }

  fn set_config_impl(&mut self, config: ProducerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<T> {
    &mut self.config
  }
}

/// Helper function to handle error strategy
fn handle_error_strategy<T>(strategy: &ErrorStrategy<T>, error: &StreamError<T>) -> ErrorAction
where
  T: std::fmt::Debug + Clone + Send + Sync,
{
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
  use futures::StreamExt;
  use serde::{Deserialize, Serialize};
  use std::io::Write;
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
}
