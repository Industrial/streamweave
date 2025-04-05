use crate::traits::{error::Error, output::Output, producer::Producer};
use futures::{Stream, StreamExt};
use std::error::Error as StdError;
use std::fmt;
use std::path::PathBuf;
use std::pin::Pin;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Debug)]
pub enum FileProducerError {
  IoError(std::io::Error),
  StreamError(String),
}

impl fmt::Display for FileProducerError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      FileProducerError::IoError(e) => write!(f, "IO error: {}", e),
      FileProducerError::StreamError(msg) => write!(f, "Stream error: {}", msg),
    }
  }
}

impl StdError for FileProducerError {
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
    match self {
      FileProducerError::IoError(e) => Some(e),
      FileProducerError::StreamError(_) => None,
    }
  }
}

pub struct FileProducer {
  path: PathBuf,
}

impl FileProducer {
  pub fn new<P: Into<PathBuf>>(path: P) -> Self {
    Self { path: path.into() }
  }
}

impl Error for FileProducer {
  type Error = FileProducerError;
}

impl Output for FileProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<String, FileProducerError>> + Send>>;
}

impl Producer for FileProducer {
  fn produce(&mut self) -> Self::OutputStream {
    let path = self.path.clone();

    let stream = async_stream::try_stream! {
        let file = File::open(&path).await.map_err(FileProducerError::IoError)?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();

        while let Ok(bytes_read) = reader.read_line(&mut line).await {
            if bytes_read == 0 {
                break;
            }
            let trimmed = line.trim().to_string();
            line.clear();
            yield trimmed;
        }
    };

    Box::pin(stream)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;
  use tempfile::NamedTempFile;
  use tokio::fs::write;

  #[tokio::test]
  async fn test_file_producer() {
    let file = NamedTempFile::new().unwrap();
    write(file.path(), "line1\nline2\nline3").await.unwrap();

    let mut producer = FileProducer::new(file.path());
    let stream = producer.produce();
    let result: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, vec!["line1", "line2", "line3"]);
  }

  #[tokio::test]
  async fn test_file_producer_empty_file() {
    let file = NamedTempFile::new().unwrap();
    let mut producer = FileProducer::new(file.path());
    let stream = producer.produce();
    let result: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, Vec::<String>::new());
  }

  #[tokio::test]
  async fn test_file_producer_nonexistent_file() {
    let mut producer = FileProducer::new("nonexistent.txt");
    let stream = producer.produce();
    let result = stream.collect::<Vec<_>>().await;
    assert!(matches!(result[0], Err(FileProducerError::IoError(_))));
  }

  #[tokio::test]
  async fn test_multiple_reads() {
    let file = NamedTempFile::new().unwrap();
    write(file.path(), "test1\ntest2").await.unwrap();

    let mut producer = FileProducer::new(file.path());

    // First read
    let stream = producer.produce();
    let result1: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result1, vec!["test1", "test2"]);

    // Second read
    let stream = producer.produce();
    let result2: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result2, vec!["test1", "test2"]);
  }

  #[tokio::test]
  async fn test_file_with_empty_lines() {
    let file = NamedTempFile::new().unwrap();
    write(file.path(), "line1\n\nline2\n\n\nline3")
      .await
      .unwrap();

    let mut producer = FileProducer::new(file.path());
    let stream = producer.produce();
    let result: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, vec!["line1", "", "line2", "", "", "line3"]);
  }
}
