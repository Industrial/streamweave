use async_trait::async_trait;
use futures::Stream;
use std::path::PathBuf;
use std::pin::Pin;
use streamweave::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave::{Output, Producer, ProducerConfig};
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::warn;

/// A producer that reads lines from a temporary file.
///
/// The temporary file is created and managed by the `tempfile` crate.
pub struct TempFileProducer {
  /// The temporary file path
  pub path: PathBuf,
  /// Configuration for the producer
  pub config: ProducerConfig<String>,
}

impl TempFileProducer {
  /// Creates a new `TempFileProducer` from a temporary file path.
  ///
  /// # Arguments
  ///
  /// * `path` - Path to the temporary file
  pub fn new(path: PathBuf) -> Self {
    Self {
      path,
      config: ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

// Trait implementations for TempFileProducer

impl Output for TempFileProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Producer for TempFileProducer {
  type OutputPorts = (String,);

  fn produce(&mut self) -> Self::OutputStream {
    let path = self.path.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "tempfile_producer".to_string());

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
                  path = %path.display(),
                  error = %e,
                  "Failed to read line from temp file, stopping"
                );
                break;
              }
            }
          }
        }
        Err(_) => {
          // File doesn't exist or can't be opened - produce empty stream
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
      ErrorStrategy::Custom(ref handler) => handler(error),
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
        .unwrap_or_else(|| "tempfile_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "tempfile_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
