use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use streamweave::{Consumer, ConsumerConfig, Input};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{error, warn};

/// A consumer that writes string items to a file.
///
/// This consumer writes each item from the stream to the specified file path.
/// The file is opened lazily when the first item is consumed.
pub struct FileConsumer {
  /// The file handle, opened when the first item is consumed.
  pub file: Option<File>,
  /// The path to the file where items will be written.
  pub path: String,
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<String>,
}

impl FileConsumer {
  /// Creates a new `FileConsumer` with the given file path.
  ///
  /// # Arguments
  ///
  /// * `path` - The path to the file where items will be written.
  pub fn new(path: String) -> Self {
    Self {
      file: None,
      path,
      config: ConsumerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this consumer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this consumer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this consumer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }
}

// Trait implementations for FileConsumer

impl Input for FileConsumer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Consumer for FileConsumer {
  type InputPorts = (String,);

  /// Consumes a stream and writes each item to the file.
  ///
  /// # Error Handling
  ///
  /// - If the file cannot be created, an error is logged and all items are dropped.
  /// - If a write fails, an error is logged but consumption continues.
  /// - If a flush fails, a warning is logged but consumption continues.
  ///
  /// Note: The configured error strategy is not currently applied to I/O errors
  /// during stream consumption. This is a known limitation of the current architecture.
  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    let component_name = self.config.name.clone();
    let path = self.path.clone();

    // Create file once at the start
    if self.file.is_none() {
      match File::create(&self.path).await {
        Ok(file) => {
          self.file = Some(file);
        }
        Err(e) => {
          error!(
            component = %component_name,
            path = %path,
            error = %e,
            "Failed to create file, all items will be dropped"
          );
        }
      }
    }

    while let Some(value) = stream.next().await {
      if let Some(file) = &mut self.file {
        if let Err(e) = file.write_all(value.as_bytes()).await {
          error!(
            component = %component_name,
            path = %path,
            error = %e,
            "Failed to write to file"
          );
        }
        // Ensure each write is flushed
        if let Err(e) = file.flush().await {
          warn!(
            component = %component_name,
            path = %path,
            error = %e,
            "Failed to flush file"
          );
        }
      }
    }

    // Final flush after stream is consumed
    if let Some(file) = &mut self.file
      && let Err(e) = file.flush().await
    {
      warn!(
        component = %component_name,
        path = %path,
        error = %e,
        "Failed to perform final flush"
      );
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
