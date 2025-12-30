use async_trait::async_trait;
use futures::Stream;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use std::pin::Pin;
use streamweave::{Output, Producer, ProducerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{error, warn};

/// A producer that reads JSON Lines (JSONL) files.
///
/// JSON Lines is a text format where each line is a valid JSON value.
/// This producer reads each line from a file and deserializes it as
/// the specified type.
///
/// # Example
///
/// ```ignore
/// use streamweave_jsonl_producer::JsonlProducer;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Event {
///     id: u32,
///     message: String,
/// }
///
/// let producer = JsonlProducer::<Event>::new("events.jsonl");
/// ```
pub struct JsonlProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// The path to the JSONL file to read from.
  pub path: String,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> JsonlProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  /// Creates a new JSONL producer for the specified file path.
  #[must_use]
  pub fn new(path: impl Into<String>) -> Self {
    Self {
      path: path.into(),
      config: ProducerConfig::default(),
      _phantom: PhantomData,
    }
  }

  /// Sets the error strategy for the producer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the producer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Returns the file path.
  #[must_use]
  pub fn path(&self) -> &str {
    &self.path
  }
}

impl<T> Clone for JsonlProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
{
  fn clone(&self) -> Self {
    Self {
      path: self.path.clone(),
      config: self.config.clone(),
      _phantom: PhantomData,
    }
  }
}

// Trait implementations for JsonlProducer

impl<T> Output for JsonlProducer<T>
where
  T: DeserializeOwned + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

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
          let mut lines = reader.lines();

          loop {
            match lines.next_line().await {
              Ok(Some(line)) => {
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
              Ok(None) => break,
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

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "jsonl_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "jsonl_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
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
