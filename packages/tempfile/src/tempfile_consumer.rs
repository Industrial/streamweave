use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::path::PathBuf;
use std::pin::Pin;
use streamweave::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave::{Consumer, ConsumerConfig, Input};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::warn;

/// A consumer that writes items to a temporary file.
///
/// The temporary file is managed externally and will be cleaned up when dropped.
pub struct TempFileConsumer {
  /// The temporary file path
  pub path: PathBuf,
  /// Configuration for the consumer
  pub config: ConsumerConfig<String>,
}

impl TempFileConsumer {
  /// Creates a new `TempFileConsumer` from a temporary file path.
  ///
  /// # Arguments
  ///
  /// * `path` - Path to the temporary file
  pub fn new(path: PathBuf) -> Self {
    Self {
      path,
      config: ConsumerConfig::default(),
    }
  }

  /// Sets the error handling strategy.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this consumer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }
}

// Trait implementations for TempFileConsumer

impl Input for TempFileConsumer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Consumer for TempFileConsumer {
  type InputPorts = (String,);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    let component_name = self.config.name.clone();
    let path = self.path.clone();

    match File::create(&path).await {
      Ok(mut file) => {
        while let Some(value) = stream.next().await {
          if let Err(e) = file.write_all(value.as_bytes()).await {
            warn!(
              component = %component_name,
              path = %path.display(),
              error = %e,
              "Failed to write to temp file"
            );
          }
          if let Err(e) = file.write_all(b"\n").await {
            warn!(
              component = %component_name,
              path = %path.display(),
              error = %e,
              "Failed to write newline to temp file"
            );
          }
        }
        if let Err(e) = file.flush().await {
          warn!(
            component = %component_name,
            path = %path.display(),
            error = %e,
            "Failed to flush temp file"
          );
        }
      }
      Err(e) => {
        warn!(
          component = %component_name,
          path = %path.display(),
          error = %e,
          "Failed to create temp file, all items will be dropped"
        );
      }
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<String> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    match self.config.error_strategy {
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
      component_name: self.config.name.clone(),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self.config.name.clone(),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
