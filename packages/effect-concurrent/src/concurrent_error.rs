use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during concurrent operations
#[derive(Error, Debug)]
pub enum ConcurrentError {
  #[error("Operation timed out after {0:?}")]
  Timeout(Duration),
  #[error("All retry attempts failed")]
  RetryExhausted,
  #[error("Resource error: {0}")]
  ResourceError(String),
}
