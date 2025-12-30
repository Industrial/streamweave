use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use streamweave::{Output, Producer, ProducerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{error, warn};

/// A producer that reads items from a file.
///
/// This producer reads lines from a file and emits each line as a string item.
pub struct FileProducer {
  /// The path to the file to read from.
  pub path: String,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<String>,
}

impl FileProducer {
  /// Creates a new `FileProducer` with the given file path.
  ///
  /// # Arguments
  ///
  /// * `path` - The path to the file to read from.
  pub fn new(path: String) -> Self {
    Self {
      path,
      config: streamweave::ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this producer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this producer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

// Trait implementations for FileProducer

impl Output for FileProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Producer for FileProducer {
  type OutputPorts = (String,);

  /// Produces a stream of lines from the file.
  ///
  /// # Error Handling
  ///
  /// - If the file cannot be opened, an error is logged and an empty stream is returned.
  /// - If a line cannot be read, a warning is logged and that line is skipped.
  ///
  /// Note: The configured error strategy is not currently applied to I/O errors
  /// during stream production. This is a known limitation of the current architecture.
  fn produce(&mut self) -> Self::OutputStream {
    let path = self.path.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "file_producer".to_string());

    Box::pin(async_stream::stream! {
      match File::open(&path).await {
        Ok(file) => {
          let reader = BufReader::new(file);
          let mut lines = reader.lines();

          loop {
            match lines.next_line().await {
              Ok(Some(line)) => yield line,
              Ok(None) => break,
              Err(e) => {
                warn!(
                  component = %component_name,
                  path = %path,
                  error = %e,
                  "Failed to read line from file, skipping"
                );
                break;
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

  fn set_config_impl(&mut self, config: ProducerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<String> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "file_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "file_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
