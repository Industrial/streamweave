use super::jsonl_consumer::JsonlConsumer;
use async_trait::async_trait;
use futures::StreamExt;
use serde::Serialize;
use streamweave_core::{Consumer, ConsumerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncWriteExt, BufWriter};
use tracing::{error, warn};

#[async_trait]
impl<T> Consumer for JsonlConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);

  /// Consumes a stream and writes each item as a JSON line to the file.
  ///
  /// # Error Handling
  ///
  /// - If the file cannot be opened, an error is logged and no data is written.
  /// - If an item cannot be serialized or written, the error strategy determines the action.
  async fn consume(&mut self, input: Self::InputStream) {
    let path = self.path.clone();
    let component_name = if self.config.name.is_empty() {
      "jsonl_consumer".to_string()
    } else {
      self.config.name.clone()
    };
    let error_strategy = self.config.error_strategy.clone();
    let append = self.jsonl_config.append;
    let buffer_size = self.jsonl_config.buffer_size;

    // Open the file
    let file_result = OpenOptions::new()
      .create(true)
      .write(true)
      .append(append)
      .truncate(!append)
      .open(&path)
      .await;

    let file = match file_result {
      Ok(f) => f,
      Err(e) => {
        error!(
          component = %component_name,
          path = %path.display(),
          error = %e,
          "Failed to open file for writing"
        );
        return;
      }
    };

    let mut writer = BufWriter::with_capacity(buffer_size, file);
    let mut input = std::pin::pin!(input);

    while let Some(item) = input.next().await {
      // Serialize the item to JSON
      let json_result = serde_json::to_string(&item);

      match json_result {
        Ok(json) => {
          // Write the JSON line
          if let Err(e) = writer.write_all(json.as_bytes()).await {
            let error = StreamError::new(
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
            match handle_error_strategy(&error_strategy, &error) {
              ErrorAction::Stop => {
                error!(
                  component = %component_name,
                  error = %error,
                  "Stopping due to write error"
                );
                break;
              }
              ErrorAction::Skip => {
                warn!(
                  component = %component_name,
                  error = %error,
                  "Skipping item due to write error"
                );
                continue;
              }
              ErrorAction::Retry => {
                warn!(
                  component = %component_name,
                  error = %error,
                  "Retry not fully supported for write errors, skipping"
                );
                continue;
              }
            }
          }

          // Write newline
          if let Err(e) = writer.write_all(b"\n").await {
            let error = StreamError::new(
              Box::new(e),
              ErrorContext {
                timestamp: chrono::Utc::now(),
                item: Some(item),
                component_name: component_name.clone(),
                component_type: std::any::type_name::<Self>().to_string(),
              },
              ComponentInfo {
                name: component_name.clone(),
                type_name: std::any::type_name::<Self>().to_string(),
              },
            );
            match handle_error_strategy(&error_strategy, &error) {
              ErrorAction::Stop => {
                error!(
                  component = %component_name,
                  error = %error,
                  "Stopping due to newline write error"
                );
                break;
              }
              ErrorAction::Skip | ErrorAction::Retry => {
                warn!(
                  component = %component_name,
                  error = %error,
                  "Error writing newline, continuing"
                );
              }
            }
          }
        }
        Err(e) => {
          let error = StreamError::new(
            Box::new(e),
            ErrorContext {
              timestamp: chrono::Utc::now(),
              item: Some(item),
              component_name: component_name.clone(),
              component_type: std::any::type_name::<Self>().to_string(),
            },
            ComponentInfo {
              name: component_name.clone(),
              type_name: std::any::type_name::<Self>().to_string(),
            },
          );
          match handle_error_strategy(&error_strategy, &error) {
            ErrorAction::Stop => {
              error!(
                component = %component_name,
                error = %error,
                "Stopping due to serialization error"
              );
              break;
            }
            ErrorAction::Skip => {
              warn!(
                component = %component_name,
                error = %error,
                "Skipping item due to serialization error"
              );
            }
            ErrorAction::Retry => {
              warn!(
                component = %component_name,
                error = %error,
                "Retry not supported for serialization errors, skipping"
              );
            }
          }
        }
      }
    }

    // Flush the writer
    if let Err(e) = writer.flush().await {
      error!(
        component = %component_name,
        error = %e,
        "Failed to flush file"
      );
    }

    // Store the file handle
    self.file = Some(writer);
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
  use proptest::prelude::*;
  use proptest::proptest;
  use serde::{Deserialize, Serialize};
  use std::io::BufRead;
  use tempfile::NamedTempFile;
  use tokio::runtime::Runtime;

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

  async fn test_jsonl_consumer_overwrite_async(
    records1: Vec<TestRecord>,
    records2: Vec<TestRecord>,
  ) {
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
    fn test_jsonl_consumer_basic(
      records in prop::collection::vec(test_record_strategy(), 0..20)
    ) {
      let rt = Runtime::new().unwrap();
      rt.block_on(test_jsonl_consumer_basic_async(records));
    }

    #[test]
    fn test_jsonl_consumer_append(
      records1 in prop::collection::vec(test_record_strategy(), 0..10),
      records2 in prop::collection::vec(test_record_strategy(), 0..10)
    ) {
      let rt = Runtime::new().unwrap();
      rt.block_on(test_jsonl_consumer_append_async(records1, records2));
    }

    #[test]
    fn test_jsonl_consumer_overwrite(
      records1 in prop::collection::vec(test_record_strategy(), 0..10),
      records2 in prop::collection::vec(test_record_strategy(), 0..10)
    ) {
      let rt = Runtime::new().unwrap();
      rt.block_on(test_jsonl_consumer_overwrite_async(records1, records2));
    }

    #[test]
    fn test_jsonl_consumer_empty_stream(_ in prop::num::u8::ANY) {
      let rt = Runtime::new().unwrap();
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
}
