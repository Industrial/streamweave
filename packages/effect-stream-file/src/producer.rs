use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use effect_core::error::EffectError;
use effect_stream::{EffectResult, EffectStream, EffectStreamSource};
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::error::FileStreamError;

/// A producer that reads from a file line by line
pub struct FileProducer {
  path: PathBuf,
  config: ProducerConfig,
}

/// Configuration for the file producer
#[derive(Clone, Debug)]
pub struct ProducerConfig {
  /// Maximum line length in bytes
  pub max_line_length: usize,
  /// Whether to trim whitespace from lines
  pub trim_lines: bool,
}

impl Default for ProducerConfig {
  fn default() -> Self {
    Self {
      max_line_length: 1024 * 1024, // 1MB
      trim_lines: true,
    }
  }
}

impl FileProducer {
  /// Create a new file producer
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
      config: ProducerConfig::default(),
    }
  }

  /// Set the configuration for this producer
  pub fn with_config(mut self, config: ProducerConfig) -> Self {
    self.config = config;
    self
  }
}

impl EffectStreamSource<String, FileStreamError> for FileProducer {
  type Stream = Pin<
    Box<
      dyn Future<Output = EffectResult<EffectStream<String, FileStreamError>, FileStreamError>>
        + Send
        + 'static,
    >,
  >;

  fn source(&self) -> Self::Stream {
    let path = self.path.clone();
    let config = self.config.clone();

    Box::pin(async move {
      let file = File::open(&path)
        .await
        .map_err(|e| FileStreamError::FileNotFound(path.to_string_lossy().to_string()))?;

      let mut reader = BufReader::new(file);
      let mut stream = EffectStream::<String, FileStreamError>::new();

      tokio::spawn(async move {
        let mut line = String::new();
        while let Ok(n) = reader.read_line(&mut line).await {
          if n == 0 {
            break;
          }

          if line.len() > config.max_line_length {
            stream
              .push_error(FileStreamError::FileTooLarge(
                path.to_string_lossy().to_string(),
              ))
              .await
              .unwrap();
            break;
          }

          let value = if config.trim_lines {
            line.trim().to_string()
          } else {
            line.clone()
          };

          stream.push(value).await.unwrap();
          line.clear();
        }

        stream.close().await.unwrap();
      });

      Ok(stream)
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::NamedTempFile;
  use tokio_test::block_on;

  #[test]
  fn test_file_producer() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "line 1").unwrap();
    writeln!(file, "line 2").unwrap();
    writeln!(file, "line 3").unwrap();

    let producer = FileProducer::new(file.path());
    let stream = block_on(producer.source()).unwrap();

    let mut values = Vec::new();
    block_on(async {
      while let Ok(Some(value)) = stream.next().await {
        values.push(value);
      }
    });

    assert_eq!(values, vec!["line 1", "line 2", "line 3"]);
  }

  #[test]
  fn test_file_producer_empty() {
    let file = NamedTempFile::new().unwrap();
    let producer = FileProducer::new(file.path());
    let stream = block_on(producer.source()).unwrap();

    let mut values = Vec::new();
    block_on(async {
      while let Ok(Some(value)) = stream.next().await {
        values.push(value);
      }
    });

    assert!(values.is_empty());
  }

  #[test]
  fn test_file_producer_not_found() {
    let producer = FileProducer::new("nonexistent.txt");
    let result = block_on(producer.source());

    assert!(matches!(result, Err(FileStreamError::FileNotFound(_))));
  }
}
