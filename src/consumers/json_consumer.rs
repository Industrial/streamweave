//! JSON consumer for writing stream data to JSON files.
//!
//! This module provides [`JsonConsumer`] and [`JsonWriteConfig`], a consumer
//! that writes stream items to JSON format. Unlike JSON Lines, this consumer
//! writes complete JSON documents (arrays or single objects).
//!
//! # Overview
//!
//! [`JsonConsumer`] is useful for exporting stream data to standard JSON format,
//! which is widely supported and human-readable. The consumer can write either:
//! - A JSON array containing all items
//! - Individual JSON objects (one per item)
//!
//! # Key Concepts
//!
//! - **JSON Format**: Standard JSON objects or arrays
//! - **Serializable Items**: Items must implement `Serialize` for JSON conversion
//! - **Array vs Object Output**: Configurable output format
//! - **Pretty Printing**: Optional human-readable formatting
//!
//! # Core Types
//!
//! - **[`JsonConsumer<T>`]**: Consumer that writes items to a JSON file
//! - **[`JsonWriteConfig`]**: Configuration for JSON writing behavior
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::consumers::JsonConsumer;
//! use futures::stream;
//! use serde::Serialize;
//!
//! #[derive(Serialize, Clone, Debug)]
//! struct Event {
//!     id: u32,
//!     message: String,
//! }
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a consumer with a file path
//! let mut consumer = JsonConsumer::<Event>::new("events.json");
//!
//! // Create a stream of events
//! let stream = stream::iter(vec![
//!     Event { id: 1, message: "event1".to_string() },
//!     Event { id: 2, message: "event2".to_string() },
//! ]);
//!
//! // Consume the stream
//! consumer.consume(stream).await?;
//! # Ok(())
//! # }
//! ```

use crate::consumer::{Consumer, ConsumerConfig};
use async_trait::async_trait;
use serde::Serialize;
use std::marker::PhantomData;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};

/// Configuration for JSON consumer behavior.
#[derive(Clone, Debug)]
pub struct JsonWriteConfig {
  /// Whether to pretty-print the JSON output.
  /// Default: true
  pub pretty: bool,
  /// Whether to write as a JSON array (true) or individual objects (false).
  /// Default: true
  pub as_array: bool,
}

impl Default for JsonWriteConfig {
  fn default() -> Self {
    Self {
      pretty: true,
      as_array: true,
    }
  }
}

/// JSON consumer that writes stream items to JSON files.
///
/// This consumer can write items as either:
/// - A JSON array containing all items
/// - Individual JSON objects (one per item)
///
/// # Type Parameters
///
/// * `T` - The type of items to consume. Must implement `Serialize`.
pub struct JsonConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Path to the JSON file.
  pub path: PathBuf,
  /// Consumer configuration.
  pub config: ConsumerConfig<T>,
  /// JSON-specific configuration.
  pub json_config: JsonWriteConfig,
  /// File handle (opened on first write).
  pub file: Option<BufWriter<File>>,
  /// Phantom data for the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> JsonConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new JSON consumer for the specified file path.
  #[must_use]
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
      config: ConsumerConfig::default(),
      json_config: JsonWriteConfig::default(),
      file: None,
      _phantom: PhantomData,
    }
  }

  /// Creates a new JSON consumer with custom configuration.
  #[must_use]
  pub fn with_config(
    path: impl Into<PathBuf>,
    config: ConsumerConfig<T>,
    json_config: JsonWriteConfig,
  ) -> Self {
    Self {
      path: path.into(),
      config,
      json_config,
      file: None,
      _phantom: PhantomData,
    }
  }
}

#[async_trait]
impl<T> Consumer for JsonConsumer<T>
where
  T: Serialize + std::fmt::Debug + Clone + Send + Sync + 'static,
{
  async fn consume(&mut self, mut stream: Self::InputStream) {
    // Open file if not already opened
    if self.file.is_none() {
      match File::create(&self.path).await {
        Ok(file) => {
          self.file = Some(BufWriter::new(file));
        }
        Err(e) => {
          let error = crate::error::StreamError::Io(e);
          match self.handle_error(&error) {
            crate::error::ErrorAction::Stop => return,
            crate::error::ErrorAction::Skip => return,
            crate::error::ErrorAction::Retry => {
              // For simplicity, just stop for now
              return;
            }
          }
        }
      }
    }

    let file = self.file.as_mut().unwrap();

    if self.json_config.as_array {
      // Write as JSON array
      if let Err(e) = file.write_all(b"[").await {
        let error = crate::error::StreamError::Io(e);
        match self.handle_error(&error) {
          crate::error::ErrorAction::Stop => return,
          crate::error::ErrorAction::Skip => return,
          crate::error::ErrorAction::Retry => {} // Continue and retry
        }
      }

      let mut first = true;
      while let Some(item) = stream.next().await {
        if !first {
          if let Err(e) = file.write_all(b",").await {
            let error = crate::error::StreamError::Io(e);
            match self.handle_error(&error) {
              crate::error::ErrorAction::Stop => return,
              crate::error::ErrorAction::Skip => continue,
              crate::error::ErrorAction::Retry => {} // Continue and retry
            }
          }
        }

        // Use pooled buffer for zero-copy JSON serialization
        let mut buffer = crate::memory_pool::global::get_buffer(1024).await;
        let json_bytes = if self.json_config.pretty {
          serde_json::to_writer_pretty(&mut buffer, &item)?;
          buffer.freeze()
        } else {
          serde_json::to_writer(&mut buffer, &item)?;
          buffer.freeze()
        };
        let json = String::from_utf8_lossy(&json_bytes);

        if let Err(e) = file.write_all(json.as_bytes()).await {
          let error = crate::error::StreamError::Io(e);
          match self.handle_error(&error) {
            crate::error::ErrorAction::Stop => return,
            crate::error::ErrorAction::Skip => continue,
            crate::error::ErrorAction::Retry => {} // Continue and retry
          }
        }

        first = false;
      }

      if let Err(e) = file.write_all(b"]").await {
        let error = crate::error::StreamError::Io(e);
        match self.handle_error(&error) {
          crate::error::ErrorAction::Stop => return,
          crate::error::ErrorAction::Skip => {}
          crate::error::ErrorAction::Retry => {} // Continue
        }
      }
    } else {
      // Write individual objects (one per line, but valid JSON)
      while let Some(item) = stream.next().await {
        // Use pooled buffer for zero-copy JSON serialization
        let mut buffer = crate::memory_pool::global::get_buffer(1024).await;
        let json_bytes = if self.json_config.pretty {
          serde_json::to_writer_pretty(&mut buffer, &item)?;
          buffer.freeze()
        } else {
          serde_json::to_writer(&mut buffer, &item)?;
          buffer.freeze()
        };
        let json = String::from_utf8_lossy(&json_bytes);

        if let Err(e) = file.write_all(json.as_bytes()).await {
          let error = crate::error::StreamError::Io(e);
          match self.handle_error(&error) {
            crate::error::ErrorAction::Stop => return,
            crate::error::ErrorAction::Skip => continue,
            crate::error::ErrorAction::Retry => {} // Continue and retry
          }
        }

        if let Err(e) = file.write_all(b"\n").await {
          let error = crate::error::StreamError::Io(e);
          match self.handle_error(&error) {
            crate::error::ErrorAction::Stop => return,
            crate::error::ErrorAction::Skip => continue,
            crate::error::ErrorAction::Retry => {} // Continue and retry
          }
        }
      }
    }

    // Flush the writer
    if let Err(e) = file.flush().await {
      let error = crate::error::StreamError::Io(e);
      match self.handle_error(&error) {
        crate::error::ErrorAction::Stop => return,
        crate::error::ErrorAction::Skip => {} // Continue
        crate::error::ErrorAction::Retry => {
          // For simplicity, just continue for now
          // In a real implementation, you might want to retry the flush
        }
      }
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<Self::Input>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<Self::Input> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<Self::Input> {
    &mut self.config
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;
  use serde::Serialize;
  use std::fs;
  use tempfile::tempdir;

  #[derive(Serialize, Clone, Debug, PartialEq)]
  struct TestItem {
    id: u32,
    name: String,
  }

  #[tokio::test]
  async fn test_json_consumer_array() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test.json");

    let mut consumer = JsonConsumer::<TestItem>::new(&file_path);
    consumer.json_config = JsonWriteConfig {
      pretty: false,
      as_array: true,
    };

    let items = vec![
      TestItem {
        id: 1,
        name: "item1".to_string(),
      },
      TestItem {
        id: 2,
        name: "item2".to_string(),
      },
    ];

    let stream = stream::iter(items.clone());
    consumer.consume(stream).await;

    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(
      content,
      r#"[{"id":1,"name":"item1"},{"id":2,"name":"item2"}]"#
    );
  }

  #[tokio::test]
  async fn test_json_consumer_pretty_array() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test_pretty.json");

    let mut consumer = JsonConsumer::<TestItem>::new(&file_path);
    consumer.json_config = JsonWriteConfig {
      pretty: true,
      as_array: true,
    };

    let items = vec![TestItem {
      id: 1,
      name: "item1".to_string(),
    }];

    let stream = stream::iter(items);
    consumer.consume(stream).await;

    let content = fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("[\n"));
    assert!(content.contains("\"id\": 1"));
  }

  #[tokio::test]
  async fn test_json_consumer_objects() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test_objects.json");

    let mut consumer = JsonConsumer::<TestItem>::new(&file_path);
    consumer.json_config = JsonWriteConfig {
      pretty: false,
      as_array: false,
    };

    let items = vec![
      TestItem {
        id: 1,
        name: "item1".to_string(),
      },
      TestItem {
        id: 2,
        name: "item2".to_string(),
      },
    ];

    let stream = stream::iter(items);
    consumer.consume(stream).await;

    let content = fs::read_to_string(&file_path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains(r#""id":1"#));
    assert!(lines[1].contains(r#""id":2"#));
  }
}
