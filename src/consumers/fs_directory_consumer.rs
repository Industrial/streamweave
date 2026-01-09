use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Consumer, ConsumerConfig, Input};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tokio::fs;
use tracing::warn;

/// A consumer that creates directories from stream items (path strings).
pub struct FsDirectoryConsumer {
  /// Configuration for the consumer
  pub config: ConsumerConfig<String>,
}

impl FsDirectoryConsumer {
  /// Creates a new `FsDirectoryConsumer`.
  pub fn new() -> Self {
    Self {
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

impl Default for FsDirectoryConsumer {
  fn default() -> Self {
    Self::new()
  }
}

impl Input for FsDirectoryConsumer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Consumer for FsDirectoryConsumer {
  type InputPorts = (String,);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    let component_name = self.config.name.clone();

    while let Some(path) = stream.next().await {
      if let Err(e) = fs::create_dir_all(&path).await {
        warn!(
          component = %component_name,
          path = %path,
          error = %e,
          "Failed to create directory"
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
      component_name: if self.config.name.is_empty() {
        "fs_directory_consumer".to_string()
      } else {
        self.config.name.clone()
      },
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
