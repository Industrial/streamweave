use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use effect_core::error::EffectError;
use effect_stream::{EffectResult, EffectStream, EffectStreamSink};
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};

use crate::error::FileStreamError;

/// A consumer that writes to a file
pub struct FileConsumer {
  path: PathBuf,
  config: ConsumerConfig,
}

/// Configuration for the file consumer
#[derive(Clone, Debug)]
pub struct ConsumerConfig {
  /// Whether to append to the file or overwrite it
  pub append: bool,
  /// Whether to create the file if it doesn't exist
  pub create: bool,
  /// Whether to create parent directories if they don't exist
  pub create_parents: bool,
}

impl Default for ConsumerConfig {
  fn default() -> Self {
    Self {
      append: false,
      create: true,
      create_parents: true,
    }
  }
}

impl FileConsumer {
  /// Create a new file consumer
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
      config: ConsumerConfig::default(),
    }
  }

  /// Set the configuration for this consumer
  pub fn with_config(mut self, config: ConsumerConfig) -> Self {
    self.config = config;
    self
  }
}

impl<T: AsRef<[u8]> + Send + 'static> EffectStreamSink<T, FileStreamError> for FileConsumer {
  type Stream = Pin<
    Box<
      dyn Future<Output = EffectResult<EffectStream<T, FileStreamError>, FileStreamError>>
        + Send
        + 'static,
    >,
  >;

  fn sink(&self) -> Self::Stream {
    let path = self.path.clone();
    let config = self.config.clone();

    Box::pin(async move {
      if config.create_parents {
        if let Some(parent) = path.parent() {
          tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| FileStreamError::FileNotWritable(parent.to_string_lossy().to_string()))?;
        }
      }

      let file = if config.create {
        tokio::fs::OpenOptions::new()
          .create(true)
          .write(true)
          .append(config.append)
          .open(&path)
          .await
          .map_err(|e| FileStreamError::FileNotWritable(path.to_string_lossy().to_string()))?
      } else {
        File::open(&path)
          .await
          .map_err(|e| FileStreamError::FileNotWritable(path.to_string_lossy().to_string()))?
      };

      let mut writer = BufWriter::new(file);
      let mut stream = EffectStream::<T, FileStreamError>::new();

      tokio::spawn(async move {
        while let Ok(Some(value)) = stream.next().await {
          if let Err(e) = writer.write_all(value.as_ref()).await {
            stream.push_error(FileStreamError::Io(e)).await.unwrap();
            break;
          }
        }

        if let Err(e) = writer.flush().await {
          stream.push_error(FileStreamError::Io(e)).await.unwrap();
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
  fn test_file_consumer() {
    let file = NamedTempFile::new().unwrap();
    let consumer = FileConsumer::new(file.path());
    let stream = block_on(consumer.sink()).unwrap();

    block_on(async {
      stream.push("line 1\n".as_bytes()).await.unwrap();
      stream.push("line 2\n".as_bytes()).await.unwrap();
      stream.push("line 3\n".as_bytes()).await.unwrap();
      stream.close().await.unwrap();
    });

    let contents = std::fs::read_to_string(file.path()).unwrap();
    assert_eq!(contents, "line 1\nline 2\nline 3\n");
  }

  #[test]
  fn test_file_consumer_append() {
    let file = NamedTempFile::new().unwrap();
    std::fs::write(file.path(), "existing\n").unwrap();

    let consumer = FileConsumer::new(file.path()).with_config(ConsumerConfig {
      append: true,
      ..Default::default()
    });
    let stream = block_on(consumer.sink()).unwrap();

    block_on(async {
      stream.push("new line\n".as_bytes()).await.unwrap();
      stream.close().await.unwrap();
    });

    let contents = std::fs::read_to_string(file.path()).unwrap();
    assert_eq!(contents, "existing\nnew line\n");
  }

  #[test]
  fn test_file_consumer_create_parents() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("subdir/file.txt");

    let consumer = FileConsumer::new(&path).with_config(ConsumerConfig {
      create_parents: true,
      ..Default::default()
    });
    let stream = block_on(consumer.sink()).unwrap();

    block_on(async {
      stream.push("test\n".as_bytes()).await.unwrap();
      stream.close().await.unwrap();
    });

    assert!(path.exists());
    let contents = std::fs::read_to_string(path).unwrap();
    assert_eq!(contents, "test\n");
  }
}
