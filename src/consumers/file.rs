use crate::traits::{consumer::Consumer, error::Error, input::Input};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::error::Error as StdError;
use std::fmt;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

#[derive(Debug)]
pub enum FileError {
  IoError(std::io::Error),
  WriteError(String),
}

impl fmt::Display for FileError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      FileError::IoError(e) => write!(f, "File IO error: {}", e),
      FileError::WriteError(msg) => write!(f, "File write error: {}", msg),
    }
  }
}

impl StdError for FileError {
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
    match self {
      FileError::IoError(e) => Some(e),
      FileError::WriteError(_) => None,
    }
  }
}

impl From<std::io::Error> for FileError {
  fn from(err: std::io::Error) -> Self {
    FileError::IoError(err)
  }
}

pub struct FileConsumer<T> {
  path: PathBuf,
  file: Option<tokio::fs::File>,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> FileConsumer<T> {
  pub fn new(path: impl AsRef<Path>) -> Self {
    Self {
      path: path.as_ref().to_path_buf(),
      file: None,
      _phantom: std::marker::PhantomData,
    }
  }

  async fn initialize(&mut self) -> Result<(), FileError> {
    if let Some(parent) = self.path.parent() {
      tokio::fs::create_dir_all(parent).await?;
    }

    let file = OpenOptions::new()
      .create(true)
      .append(true)
      .open(&self.path)
      .await?;

    self.file = Some(file);
    Ok(())
  }
}

impl<T: ToString + Send + 'static> Error for FileConsumer<T> {
  type Error = FileError;
}

impl<T: ToString + Send + 'static> Input for FileConsumer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, Self::Error>> + Send>>;
}

#[async_trait]
impl<T: ToString + Send + 'static> Consumer for FileConsumer<T> {
  async fn consume(&mut self, mut stream: Self::InputStream) -> Result<(), Self::Error> {
    self.initialize().await?;

    let file = self
      .file
      .as_mut()
      .ok_or_else(|| FileError::WriteError("File not initialized".to_string()))?;

    while let Some(item) = stream.next().await {
      let line = format!("{}\n", item?.to_string());
      file.write_all(line.as_bytes()).await?;
      file.flush().await?;
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;
  use tempfile::tempdir;

  #[tokio::test]
  async fn test_file_consumer() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    let mut consumer = FileConsumer::new(&file_path);
    let items = vec![
      "test1".to_string(),
      "test2".to_string(),
      "test3".to_string(),
    ];
    let stream = Box::pin(stream::iter(items.clone()).map(Ok));

    assert!(consumer.consume(stream).await.is_ok());

    // Verify file contents
    let contents = tokio::fs::read_to_string(&file_path).await.unwrap();
    let expected = items.join("\n") + "\n";
    assert_eq!(contents, expected);
  }

  #[tokio::test]
  async fn test_multiple_writes() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test_multiple.txt");
    let mut consumer = FileConsumer::new(&file_path);

    // First write
    let items1 = vec!["first1".to_string(), "first2".to_string()];
    let stream1 = Box::pin(stream::iter(items1.clone()).map(Ok));
    assert!(consumer.consume(stream1).await.is_ok());

    // Second write
    let items2 = vec!["second1".to_string(), "second2".to_string()];
    let stream2 = Box::pin(stream::iter(items2.clone()).map(Ok));
    assert!(consumer.consume(stream2).await.is_ok());

    // Verify file contents
    let contents = tokio::fs::read_to_string(&file_path).await.unwrap();
    let expected = format!("{}\n{}\n", items1.join("\n"), items2.join("\n"));
    assert_eq!(contents, expected);
  }

  #[tokio::test]
  async fn test_error_handling() {
    let temp_dir = tempdir().unwrap();
    let file_path = temp_dir.path().join("test_error.txt");
    let mut consumer = FileConsumer::new(&file_path);

    let error_stream = Box::pin(stream::iter(vec![
      Ok("line1".to_string()),
      Err(FileError::WriteError("test error".to_string())),
      Ok("line3".to_string()),
    ]));

    let result = consumer.consume(error_stream).await;
    assert!(result.is_err());
    match result {
      Err(FileError::WriteError(msg)) => assert_eq!(msg, "test error"),
      _ => panic!("Expected WriteError"),
    }
  }

  #[tokio::test]
  async fn test_invalid_path() {
    let mut consumer = FileConsumer::<String>::new("/nonexistent/directory/file.txt");
    let stream = Box::pin(stream::iter(vec!["test".to_string()]).map(Ok));

    let result = consumer.consume(stream).await;
    assert!(matches!(result, Err(FileError::IoError(_))));
  }
}
