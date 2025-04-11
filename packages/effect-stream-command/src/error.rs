use std::fmt;
use thiserror::Error;

/// Errors that can occur when working with command streams
#[derive(Debug, Error)]
pub enum CommandStreamError {
  /// The command failed to start
  #[error("Failed to start command: {0}")]
  CommandStart(String),

  /// The command failed to execute
  #[error("Command execution failed: {0}")]
  CommandExecution(String),

  /// The command produced no output
  #[error("Command produced no output: {0}")]
  NoOutput(String),

  /// The command timed out
  #[error("Command timed out: {0}")]
  Timeout(String),

  /// An I/O error occurred
  #[error("I/O error: {0}")]
  Io(#[from] std::io::Error),
}

impl Clone for CommandStreamError {
  fn clone(&self) -> Self {
    match self {
      Self::CommandStart(msg) => Self::CommandStart(msg.clone()),
      Self::CommandExecution(msg) => Self::CommandExecution(msg.clone()),
      Self::NoOutput(msg) => Self::NoOutput(msg.clone()),
      Self::Timeout(msg) => Self::Timeout(msg.clone()),
      Self::Io(e) => Self::Io(std::io::Error::new(e.kind(), e.to_string())),
    }
  }
}

impl PartialEq for CommandStreamError {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Self::CommandStart(a), Self::CommandStart(b)) => a == b,
      (Self::CommandExecution(a), Self::CommandExecution(b)) => a == b,
      (Self::NoOutput(a), Self::NoOutput(b)) => a == b,
      (Self::Timeout(a), Self::Timeout(b)) => a == b,
      _ => false,
    }
  }
}
