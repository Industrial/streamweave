use super::jsonl_consumer::JsonlConsumer;
use crate::consumer::{Consumer, ConsumerConfig};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use async_trait::async_trait;
use futures::StreamExt;
use serde::Serialize;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncWriteExt, BufWriter};
use tracing::{error, warn};

#[async_trait]
impl<T> Consumer for JsonlConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
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
  use serde::{Deserialize, Serialize};
  use std::io::BufRead;
  use tempfile::NamedTempFile;

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  struct TestRecord {
    id: u32,
    name: String,
  }

  #[tokio::test]
  async fn test_jsonl_consumer_basic() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap().to_string();

    let records = vec![
      TestRecord {
        id: 1,
        name: "Alice".to_string(),
      },
      TestRecord {
        id: 2,
        name: "Bob".to_string(),
      },
    ];

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

  #[tokio::test]
  async fn test_jsonl_consumer_append() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap().to_string();

    // Write first batch
    let records1 = vec![TestRecord {
      id: 1,
      name: "Alice".to_string(),
    }];

    let mut consumer1 = JsonlConsumer::new(&path);
    let input_stream1 = Box::pin(stream::iter(records1));
    consumer1.consume(input_stream1).await;

    // Append second batch
    let records2 = vec![TestRecord {
      id: 2,
      name: "Bob".to_string(),
    }];

    let mut consumer2 = JsonlConsumer::new(&path).with_append(true);
    let input_stream2 = Box::pin(stream::iter(records2));
    consumer2.consume(input_stream2).await;

    // Read and verify the file
    let file_content = std::fs::File::open(&path).unwrap();
    let reader = std::io::BufReader::new(file_content);
    let lines: Vec<TestRecord> = reader
      .lines()
      .map(|l| serde_json::from_str(&l.unwrap()).unwrap())
      .collect();

    assert_eq!(
      lines,
      vec![
        TestRecord {
          id: 1,
          name: "Alice".to_string()
        },
        TestRecord {
          id: 2,
          name: "Bob".to_string()
        },
      ]
    );
    drop(file);
  }

  #[tokio::test]
  async fn test_jsonl_consumer_overwrite() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap().to_string();

    // Write first batch
    let records1 = vec![
      TestRecord {
        id: 1,
        name: "Alice".to_string(),
      },
      TestRecord {
        id: 2,
        name: "Bob".to_string(),
      },
    ];

    let mut consumer1 = JsonlConsumer::new(&path);
    let input_stream1 = Box::pin(stream::iter(records1));
    consumer1.consume(input_stream1).await;

    // Overwrite with second batch
    let records2 = vec![TestRecord {
      id: 3,
      name: "Charlie".to_string(),
    }];

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

  #[tokio::test]
  async fn test_jsonl_consumer_empty_stream() {
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

  #[tokio::test]
  async fn test_jsonl_consumer_component_info() {
    let consumer =
      JsonlConsumer::<TestRecord>::new("test.jsonl").with_name("my_jsonl_consumer".to_string());
    let info = consumer.component_info();
    assert_eq!(info.name, "my_jsonl_consumer");
    assert_eq!(
      info.type_name,
      std::any::type_name::<JsonlConsumer<TestRecord>>()
    );
  }

  #[tokio::test]
  async fn test_jsonl_consumer_create_error_context() {
    let consumer =
      JsonlConsumer::<TestRecord>::new("test.jsonl").with_name("test_consumer".to_string());
    let item = TestRecord {
      id: 1,
      name: "Test".to_string(),
    };
    let ctx = consumer.create_error_context(Some(item.clone()));
    assert_eq!(ctx.component_name, "test_consumer");
    assert_eq!(ctx.item, Some(item));
  }
}
