use super::csv_consumer::CsvConsumer;
use crate::consumer::{Consumer, ConsumerConfig};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use async_trait::async_trait;
use csv::WriterBuilder;
use futures::StreamExt;
use serde::Serialize;
use std::fs::File;
use tracing::{error, warn};

#[async_trait]
impl<T> Consumer for CsvConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Consumes a stream and writes each item as a CSV row to the file.
  ///
  /// # Error Handling
  ///
  /// - If the file cannot be opened, an error is logged and no data is written.
  /// - If an item cannot be serialized or written, the error strategy determines the action.
  async fn consume(&mut self, input: Self::InputStream) {
    let path = self.path.clone();
    let component_name = if self.config.name.is_empty() {
      "csv_consumer".to_string()
    } else {
      self.config.name.clone()
    };
    let error_strategy = self.config.error_strategy.clone();
    let csv_config = self.csv_config.clone();

    // Open the file
    let file = match File::create(&path) {
      Ok(f) => f,
      Err(e) => {
        error!(
          component = %component_name,
          path = %path.display(),
          error = %e,
          "Failed to create CSV file for writing"
        );
        return;
      }
    };

    // Create CSV writer
    let mut writer = WriterBuilder::new()
      .has_headers(csv_config.write_headers)
      .delimiter(csv_config.delimiter)
      .quote(csv_config.quote)
      .double_quote(csv_config.double_quote)
      .from_writer(file);

    let mut input = std::pin::pin!(input);

    while let Some(item) = input.next().await {
      // Serialize the record
      if let Err(e) = writer.serialize(&item) {
        let stream_error = StreamError::new(
          Box::new(e),
          ErrorContext {
            timestamp: chrono::Utc::now(),
            item: Some(item.clone()),
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
              "Stopping due to serialization error"
            );
            break;
          }
          ErrorAction::Skip => {
            warn!(
              component = %component_name,
              error = %stream_error,
              "Skipping item due to serialization error"
            );
            continue;
          }
          ErrorAction::Retry => {
            warn!(
              component = %component_name,
              error = %stream_error,
              "Retry not supported for CSV serialization errors, skipping"
            );
            continue;
          }
        }
      }

      // Flush if configured
      if csv_config.flush_on_write
        && let Err(e) = writer.flush()
      {
        warn!(
          component = %component_name,
          error = %e,
          "Failed to flush CSV writer"
        );
      }
    }

    // Final flush
    if let Err(e) = writer.flush() {
      error!(
        component = %component_name,
        error = %e,
        "Failed to flush CSV file"
      );
    }

    self.first_record_written = true;
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<T> {
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
  use futures::stream;
  use serde::{Deserialize, Serialize};
  use tempfile::NamedTempFile;

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  struct TestRecord {
    name: String,
    age: u32,
  }

  #[tokio::test]
  async fn test_csv_consumer_basic() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap().to_string();

    let records = vec![
      TestRecord {
        name: "Alice".to_string(),
        age: 30,
      },
      TestRecord {
        name: "Bob".to_string(),
        age: 25,
      },
    ];

    let mut consumer = CsvConsumer::new(&path);
    let input_stream = Box::pin(stream::iter(records));
    consumer.consume(input_stream).await;

    // Read and verify the file
    let content = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    assert_eq!(lines.len(), 3); // Header + 2 records
    assert!(lines[0].contains("name"));
    assert!(lines[0].contains("age"));
    assert!(lines[1].contains("Alice"));
    assert!(lines[2].contains("Bob"));
    drop(file);
  }

  #[tokio::test]
  async fn test_csv_consumer_no_headers() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap().to_string();

    let records = vec![TestRecord {
      name: "Alice".to_string(),
      age: 30,
    }];

    let mut consumer = CsvConsumer::new(&path).with_headers(false);
    let input_stream = Box::pin(stream::iter(records));
    consumer.consume(input_stream).await;

    // Read and verify the file
    let content = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    // No header row, just the data
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("Alice"));
    drop(file);
  }

  #[tokio::test]
  async fn test_csv_consumer_tab_delimited() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap().to_string();

    let records = vec![TestRecord {
      name: "Alice".to_string(),
      age: 30,
    }];

    let mut consumer = CsvConsumer::new(&path).with_delimiter(b'\t');
    let input_stream = Box::pin(stream::iter(records));
    consumer.consume(input_stream).await;

    // Read and verify the file
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains('\t'));
    drop(file);
  }

  #[tokio::test]
  async fn test_csv_consumer_empty_stream() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap().to_string();

    let mut consumer = CsvConsumer::<TestRecord>::new(&path);
    let input_stream = Box::pin(stream::iter(Vec::<TestRecord>::new()));
    consumer.consume(input_stream).await;

    // Read and verify the file - should have header only
    let content = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert!(lines.len() <= 1); // Header only or empty
    drop(file);
  }

  #[tokio::test]
  async fn test_csv_consumer_component_info() {
    let consumer =
      CsvConsumer::<TestRecord>::new("test.csv").with_name("my_csv_consumer".to_string());
    let info = consumer.component_info();
    assert_eq!(info.name, "my_csv_consumer");
    assert_eq!(
      info.type_name,
      std::any::type_name::<CsvConsumer<TestRecord>>()
    );
  }

  #[tokio::test]
  async fn test_csv_roundtrip() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap().to_string();

    // Write CSV
    let records = vec![
      TestRecord {
        name: "Alice".to_string(),
        age: 30,
      },
      TestRecord {
        name: "Bob".to_string(),
        age: 25,
      },
    ];

    let mut consumer = CsvConsumer::new(&path);
    let input_stream = Box::pin(stream::iter(records.clone()));
    consumer.consume(input_stream).await;

    // Read CSV back using csv crate
    let read_file = std::fs::File::open(&path).unwrap();
    let mut reader = csv::Reader::from_reader(read_file);
    let read_records: Vec<TestRecord> = reader.deserialize().filter_map(|r| r.ok()).collect();

    assert_eq!(read_records, records);
    drop(file);
  }
}
