use std::fmt;
use thiserror::Error;

/// Errors that can occur when working with file streams
#[derive(Debug, Error)]
pub enum FileStreamError {
  /// An I/O error occurred
  #[error("I/O error: {0}")]
  Io(#[from] std::io::Error),

  /// The file does not exist
  #[error("File not found: {0}")]
  FileNotFound(String),

  /// The file is not readable
  #[error("File not readable: {0}")]
  FileNotReadable(String),

  /// The file is not writable
  #[error("File not writable: {0}")]
  FileNotWritable(String),

  /// The file is empty
  #[error("File is empty: {0}")]
  FileEmpty(String),

  /// The file is too large
  #[error("File too large: {0}")]
  FileTooLarge(String),
}

impl Clone for FileStreamError {
  fn clone(&self) -> Self {
    match self {
      Self::Io(e) => Self::Io(std::io::Error::new(e.kind(), e.to_string())),
      Self::FileNotFound(path) => Self::FileNotFound(path.clone()),
      Self::FileNotReadable(path) => Self::FileNotReadable(path.clone()),
      Self::FileNotWritable(path) => Self::FileNotWritable(path.clone()),
      Self::FileEmpty(path) => Self::FileEmpty(path.clone()),
      Self::FileTooLarge(path) => Self::FileTooLarge(path.clone()),
    }
  }
}

impl PartialEq for FileStreamError {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Self::FileNotFound(a), Self::FileNotFound(b)) => a == b,
      (Self::FileNotReadable(a), Self::FileNotReadable(b)) => a == b,
      (Self::FileNotWritable(a), Self::FileNotWritable(b)) => a == b,
      (Self::FileEmpty(a), Self::FileEmpty(b)) => a == b,
      (Self::FileTooLarge(a), Self::FileTooLarge(b)) => a == b,
      _ => false,
    }
  }
}
