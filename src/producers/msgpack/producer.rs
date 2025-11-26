use super::msgpack_producer::MsgPackProducer;
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::producer::{Producer, ProducerConfig};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use std::fs::File;
use std::io::BufReader;
use tracing::error;

#[async_trait]
impl<T> Producer for MsgPackProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// Produces a stream of deserialized MessagePack objects from a file.
  ///
  /// # Error Handling
  ///
  /// - If the file cannot be opened, an error is logged and an empty stream is returned.
  /// - If an item cannot be deserialized, the error strategy determines the action.
  fn produce(&mut self) -> Self::OutputStream {
    let path = self.path.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "msgpack_producer".to_string());
    let error_strategy = self.config.error_strategy.clone();

    // MessagePack reading is synchronous, so we use blocking_task wrapper
    Box::pin(async_stream::stream! {
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
              "Failed to open MessagePack file"
            );
            return;
          }
        };

        let reader = BufReader::new(file);
        let mut deserializer = rmp_serde::Deserializer::new(reader);

        loop {
          match serde::de::Deserialize::deserialize(&mut deserializer) {
            Ok(item) => {
              if tx.blocking_send(Ok(item)).is_err() {
                break;
              }
            }
            Err(e) => {
              // Check if we've reached EOF
              if e.to_string().contains("EOF") || e.to_string().contains("end of file") {
                break;
              }

              let stream_error = StreamError::new(
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())),
                ErrorContext {
                  timestamp: chrono::Utc::now(),
                  item: None,
                  component_name: component_name_clone.clone(),
                  component_type: std::any::type_name::<MsgPackProducer<T>>().to_string(),
                },
                ComponentInfo {
                  name: component_name_clone.clone(),
                  type_name: std::any::type_name::<MsgPackProducer<T>>().to_string(),
                },
              );

              match handle_error_strategy(&error_strategy_clone, &stream_error) {
                ErrorAction::Stop => {
                  let _ = tx.blocking_send(Err(stream_error));
                  break;
                }
                ErrorAction::Skip => {
                  // Skip this item and try next
                  continue;
                }
                ErrorAction::Retry => {
                  // Retry not supported, skip
                  continue;
                }
              }
            }
          }
        }
      });

      while let Some(result) = rx.recv().await {
        match result {
          Ok(item) => yield item,
          Err(stream_error) => {
            error!(
              component = %component_name,
              error = %stream_error,
              "Error reading MessagePack item"
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
  struct TestData {
    id: u32,
    name: String,
  }

  #[tokio::test]
  async fn test_msgpack_producer_basic() {
    let mut file = NamedTempFile::new().unwrap();

    // Write multiple MessagePack items
    let items = vec![
      TestData {
        id: 1,
        name: "Alice".to_string(),
      },
      TestData {
        id: 2,
        name: "Bob".to_string(),
      },
    ];

    for item in &items {
      let bytes = rmp_serde::to_vec(item).unwrap();
      file.write_all(&bytes).unwrap();
    }
    file.flush().unwrap();
    let path = file.path().to_str().unwrap().to_string();

    let mut producer = MsgPackProducer::<TestData>::new(path);
    let stream = producer.produce();
    let result: Vec<TestData> = stream.collect().await;

    assert_eq!(result, items);
    drop(file);
  }

  #[tokio::test]
  async fn test_msgpack_producer_empty_file() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap().to_string();

    let mut producer = MsgPackProducer::<TestData>::new(path);
    let stream = producer.produce();
    let result: Vec<TestData> = stream.collect().await;

    assert!(result.is_empty());
    drop(file);
  }

  #[tokio::test]
  async fn test_msgpack_producer_nonexistent_file() {
    let mut producer = MsgPackProducer::<TestData>::new("nonexistent.msgpack".to_string());
    let stream = producer.produce();
    let result: Vec<TestData> = stream.collect().await;

    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_msgpack_producer_component_info() {
    let producer =
      MsgPackProducer::<TestData>::new("path.msgpack").with_name("my_msgpack_producer".to_string());
    let info = producer.component_info();
    assert_eq!(info.name, "my_msgpack_producer");
    assert_eq!(
      info.type_name,
      std::any::type_name::<MsgPackProducer<TestData>>()
    );
  }
}
