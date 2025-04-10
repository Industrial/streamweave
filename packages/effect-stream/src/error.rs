//! Stream-specific error types and utilities.
//!
//! This module provides error types and utilities for working with streams.
//! It includes common stream errors and error handling utilities.

use std::error::Error as StdError;
use thiserror::Error;

/// A unified error type for both stream operations and custom errors
#[derive(Error, Debug, Clone, PartialEq)]
pub enum EffectError<E> {
  #[error("Stream processing error: {0}")]
  Processing(String),

  #[error("Stream is closed")]
  Closed,

  #[error("Read error: {0}")]
  Read(String),

  #[error("Write error: {0}")]
  Write(String),

  #[error("Custom error: {0}")]
  Custom(E),
}

impl<E: StdError> EffectError<E> {
  /// Create a new processing error
  pub fn processing(msg: impl Into<String>) -> Self {
    Self::Processing(msg.into())
  }

  /// Create a new read error
  pub fn read(msg: impl Into<String>) -> Self {
    Self::Read(msg.into())
  }

  /// Create a new write error
  pub fn write(msg: impl Into<String>) -> Self {
    Self::Write(msg.into())
  }
}

impl<E: StdError> From<E> for EffectError<E> {
  fn from(err: E) -> Self {
    Self::Custom(err)
  }
}

/// A type alias for results that can fail with an EffectError
pub type EffectResult<T, E> = Result<T, EffectError<E>>;
