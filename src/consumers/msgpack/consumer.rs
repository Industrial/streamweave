use super::msgpack_consumer::MsgPackConsumer;
use crate::consumer::{Consumer, ConsumerConfig};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use async_trait::async_trait;
use futures::StreamExt;
use serde::Serialize;
use std::fs::File;
use std::io::{BufWriter, Write};
use tracing::{error, warn};

#[async_trait]
impl<T> Consumer for MsgPackConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Consumes a stream and writes each item as MessagePack to the file.
  ///
  /// # Error Handling
  ///
  /// - If the file cannot be opened, an error is logged and no data is written.
  /// - If an item cannot be serialized or written, the error strategy determines the action.
  async fn consume(&mut self, input: Self::InputStream) {
    let path = self.path.clone();
    let component_name = if self.config.name.is_empty() {
      "msgpack_consumer".to_string()
    } else {
      self.config.name.clone()
    };
    let error_strategy = self.config.error_strategy.clone();

    // Open the file
    let file = match File::create(&path) {
      Ok(f) => f,
      Err(e) => {
        error!(
          component = %component_name,
          path = %path.display(),
          error = %e,
          "Failed to create MessagePack file for writing"
        );
        return;
      }
    };

    let mut writer = BufWriter::new(file);
    let mut input = std::pin::pin!(input);

    while let Some(item) = input.next().await {
      // Serialize the item to MessagePack
      let bytes = match rmp_serde::to_vec(&item) {
        Ok(b) => b,
        Err(e) => {
          let stream_error = StreamError::new(
            Box::new(std::io::Error::new(
              std::io::ErrorKind::InvalidData,
              e.to_string(),
            )),
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
                "Retry not supported for serialization errors, skipping"
              );
              continue;
            }
          }
        }
      };

      // Write the bytes
      if let Err(e) = writer.write_all(&bytes) {
        let stream_error = StreamError::new(
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
              "Skipping item due to write error"
            );
            continue;
          }
          ErrorAction::Retry => {
            warn!(
              component = %component_name,
              error = %stream_error,
              "Retry not supported for write errors, skipping"
            );
            continue;
          }
        }
      }
    }

    // Flush the writer
    if let Err(e) = writer.flush() {
      error!(
        component = %component_name,
        error = %e,
        "Failed to flush MessagePack file"
      );
    }

    self.writer = Some(writer);
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
  use std::io::BufReader;
  use tempfile::NamedTempFile;

  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
  struct TestRecord {
    name: String,
    age: u32,
  }

  #[tokio::test]
  async fn test_msgpack_consumer_basic() {
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

    let mut consumer = MsgPackConsumer::new(&path);
    let input_stream = Box::pin(stream::iter(records.clone()));
    consumer.consume(input_stream).await;

    // Read and verify the file
    let read_file = File::open(&path).unwrap();
    let reader = BufReader::new(read_file);
    let mut deserializer = rmp_serde::Deserializer::new(reader);

    let mut result: Vec<TestRecord> = Vec::new();
    while let Ok(item) = serde::de::Deserialize::deserialize(&mut deserializer) {
      result.push(item);
    }

    assert_eq!(result, records);
    drop(file);
  }

  #[tokio::test]
  async fn test_msgpack_consumer_empty_stream() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap().to_string();

    let mut consumer = MsgPackConsumer::<TestRecord>::new(&path);
    let input_stream = Box::pin(stream::iter(Vec::<TestRecord>::new()));
    consumer.consume(input_stream).await;

    // File should be empty
    let content_len = std::fs::metadata(&path).unwrap().len();
    assert_eq!(content_len, 0);
    drop(file);
  }

  #[tokio::test]
  async fn test_msgpack_consumer_component_info() {
    let consumer = MsgPackConsumer::<TestRecord>::new("test.msgpack")
      .with_name("my_msgpack_consumer".to_string());
    let info = consumer.component_info();
    assert_eq!(info.name, "my_msgpack_consumer");
    assert_eq!(
      info.type_name,
      std::any::type_name::<MsgPackConsumer<TestRecord>>()
    );
  }

  #[tokio::test]
  async fn test_msgpack_roundtrip() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap().to_string();

    // Write
    let original = vec![
      TestRecord {
        name: "Alice".to_string(),
        age: 30,
      },
      TestRecord {
        name: "Bob".to_string(),
        age: 25,
      },
    ];

    let mut consumer = MsgPackConsumer::new(&path);
    let input_stream = Box::pin(stream::iter(original.clone()));
    consumer.consume(input_stream).await;

    // Read back
    let read_file = File::open(&path).unwrap();
    let reader = BufReader::new(read_file);
    let mut deserializer = rmp_serde::Deserializer::new(reader);

    let mut result: Vec<TestRecord> = Vec::new();
    while let Ok(item) = serde::de::Deserialize::deserialize(&mut deserializer) {
      result.push(item);
    }

    assert_eq!(result, original);
    drop(file);
  }
}
