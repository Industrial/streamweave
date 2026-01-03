use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Output, Producer, ProducerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio::fs;

/// A producer that reads directory entries from a directory path.
///
/// Emits paths to files and subdirectories found in the specified directory.
pub struct FsDirectoryProducer {
  /// The directory path to read from
  pub path: String,
  /// Whether to read recursively
  pub recursive: bool,
  /// Configuration for the producer
  pub config: ProducerConfig<String>,
}

impl FsDirectoryProducer {
  /// Creates a new `FsDirectoryProducer` with the given directory path.
  ///
  /// # Arguments
  ///
  /// * `path` - The directory path to read from
  pub fn new(path: String) -> Self {
    Self {
      path,
      recursive: false,
      config: ProducerConfig::default(),
    }
  }

  /// Sets whether to read directories recursively.
  pub fn recursive(mut self, recursive: bool) -> Self {
    self.recursive = recursive;
    self
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

impl Output for FsDirectoryProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
#[async_trait]
impl Producer for FsDirectoryProducer {
  type OutputPorts = (String,);

  fn produce(&mut self) -> Self::OutputStream {
    let path = self.path.clone();
    let recursive = self.recursive;

    Box::pin(async_stream::stream! {
      match fs::read_dir(&path).await {
        Ok(mut entries) => {
          if recursive {
            // Recursive directory walk
            let mut stack = vec![path];
            while let Some(dir_path) = stack.pop() {
              match fs::read_dir(&dir_path).await {
                Ok(mut dir_entries) => {
                  while let Ok(Some(entry)) = dir_entries.next_entry().await {
                    if let Some(path_str) = entry.path().to_str() {
                      let path = path_str.to_string();
                      yield path.clone();
                      if entry.file_type().await.map(|ft| ft.is_dir()).unwrap_or(false) {
                        stack.push(path);
                      }
                    }
                  }
                }
                Err(_) => {
                  // Skip directories that can't be read
                }
              }
            }
          } else {
            // Non-recursive: just yield entries in the top-level directory
            while let Ok(Some(entry)) = entries.next_entry().await {
              if let Some(path_str) = entry.path().to_str() {
                yield path_str.to_string();
              }
            }
          }
        }
        Err(_) => {
          // Directory doesn't exist or can't be read - produce empty stream
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
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "fs_directory_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "fs_directory_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
