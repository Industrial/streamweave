use super::jsonl_producer::JsonlProducer;
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::producer::{Producer, ProducerConfig};
use async_trait::async_trait;
use futures::StreamExt;
use serde::de::DeserializeOwned;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_stream::wrappers::LinesStream;
use tracing::{error, warn};

#[async_trait]
impl<T> Producer for JsonlProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  type OutputPorts = (T,);

  /// Produces a stream of deserialized JSON objects from a JSON Lines file.
  ///
  /// # Error Handling
  ///
  /// - If the file cannot be opened, an error is logged and an empty stream is returned.
  /// - If a line cannot be read or deserialized, the error strategy determines the action.
  fn produce(&mut self) -> Self::OutputStream {
    let path = self.path.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "jsonl_producer".to_string());
    let error_strategy = self.config.error_strategy.clone();

    Box::pin(async_stream::stream! {
      match File::open(&path).await {
        Ok(file) => {
          let reader = BufReader::new(file);
          let mut lines = LinesStream::new(reader.lines());

          while let Some(line_result) = lines.next().await {
            match line_result {
              Ok(line) => {
                match serde_json::from_str::<T>(&line) {
                  Ok(item) => yield item,
                  Err(e) => {
                    let error = StreamError::new(
                      Box::new(e),
                      ErrorContext {
                        timestamp: chrono::Utc::now(),
                        item: None, // Cannot provide item if deserialization failed
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
                          "Stopping due to deserialization error"
                        );
                        break;
                      }
                      ErrorAction::Skip => {
                        warn!(
                          component = %component_name,
                          error = %error,
                          "Skipping item due to deserialization error"
                        );
                      }
                      ErrorAction::Retry => {
                        // Retry logic is not directly applicable here for a single item.
                        // For now, treat as skip or stop based on policy.
                        warn!(
                          component = %component_name,
                          error = %error,
                          "Retry strategy not fully supported for single item deserialization errors, skipping"
                        );
                      }
                    }
                  }
                }
              }
              Err(e) => {
                let error = StreamError::new(
                  Box::new(e),
                  ErrorContext {
                    timestamp: chrono::Utc::now(),
                    item: None,
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
                      "Stopping due to line read error"
                    );
                    break;
                  }
                  ErrorAction::Skip => {
                    warn!(
                      component = %component_name,
                      error = %error,
                      "Skipping line due to read error"
                    );
                  }
                  ErrorAction::Retry => {
                    warn!(
                      component = %component_name,
                      error = %error,
                      "Retry strategy not fully supported for line read errors, skipping"
                    );
                  }
                }
              }
            }
          }
        }
        Err(e) => {
          error!(
            component = %component_name,
            path = %path,
            error = %e,
            "Failed to open file, producing empty stream"
          );
        }
      }
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
  async fn test_jsonl_producer_basic() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, r#"{{"id": 1, "name": "Alice"}}"#).unwrap();
    writeln!(file, r#"{{"id": 2, "name": "Bob"}}"#).unwrap();
    file.flush().unwrap();
    let path = file.path().to_str().unwrap().to_string();

    let mut producer = JsonlProducer::<TestData>::new(path);
    let stream = producer.produce();
    let result: Vec<TestData> = stream.collect().await;

    assert_eq!(
      result,
      vec![
        TestData {
          id: 1,
          name: "Alice".to_string()
        },
        TestData {
          id: 2,
          name: "Bob".to_string()
        },
      ]
    );
    drop(file);
  }

  #[tokio::test]
  async fn test_jsonl_producer_empty_file() {
    let file = NamedTempFile::new().unwrap();
    let path = file.path().to_str().unwrap().to_string();

    let mut producer = JsonlProducer::<TestData>::new(path);
    let stream = producer.produce();
    let result: Vec<TestData> = stream.collect().await;

    assert!(result.is_empty());
    drop(file);
  }

  #[tokio::test]
  async fn test_jsonl_producer_nonexistent_file() {
    let mut producer = JsonlProducer::<TestData>::new("nonexistent.jsonl".to_string());
    let stream = producer.produce();
    let result: Vec<TestData> = stream.collect().await;

    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_jsonl_producer_malformed_json_skip() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, r#"{{"id": 1, "name": "Alice"}}"#).unwrap();
    writeln!(file, r#"malformed json"#).unwrap();
    writeln!(file, r#"{{"id": 2, "name": "Bob"}}"#).unwrap();
    file.flush().unwrap();
    let path = file.path().to_str().unwrap().to_string();

    // Default error strategy is Stop, so only first item will be produced
    let mut producer =
      JsonlProducer::<TestData>::new(path).with_error_strategy(ErrorStrategy::Skip);
    let stream = producer.produce();
    let result: Vec<TestData> = stream.collect().await;

    // Both valid items should be produced
    assert_eq!(
      result,
      vec![
        TestData {
          id: 1,
          name: "Alice".to_string()
        },
        TestData {
          id: 2,
          name: "Bob".to_string()
        },
      ]
    );
    drop(file);
  }

  #[tokio::test]
  async fn test_jsonl_producer_error_strategy_stop() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, r#"{{"id": 1, "name": "Alice"}}"#).unwrap();
    writeln!(file, r#"malformed json"#).unwrap(); // This should stop the stream
    writeln!(file, r#"{{"id": 2, "name": "Bob"}}"#).unwrap();
    file.flush().unwrap();
    let path = file.path().to_str().unwrap().to_string();

    let mut producer =
      JsonlProducer::<TestData>::new(path).with_error_strategy(ErrorStrategy::Stop);
    let stream = producer.produce();
    let result: Vec<TestData> = stream.collect().await;

    // Only the first valid item should be produced before stopping
    assert_eq!(
      result,
      vec![TestData {
        id: 1,
        name: "Alice".to_string()
      }]
    );
    drop(file);
  }

  #[tokio::test]
  async fn test_jsonl_producer_component_info() {
    let producer =
      JsonlProducer::<TestData>::new("path.jsonl").with_name("my_jsonl_producer".to_string());
    let info = producer.component_info();
    assert_eq!(info.name, "my_jsonl_producer");
    assert_eq!(
      info.type_name,
      std::any::type_name::<JsonlProducer<TestData>>()
    );
  }
}
