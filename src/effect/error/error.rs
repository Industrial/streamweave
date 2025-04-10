use std::{
  error::Error as StdError,
  fmt::{Debug, Display, Formatter},
  io::Error as IoError,
};

/// A custom error type that implements PartialEq for use with Effect
#[derive(Debug)]
pub struct EffectError {
  kind: ErrorKind,
  message: String,
  source: Option<Box<dyn StdError + Send + Sync>>,
}

impl PartialEq for EffectError {
  fn eq(&self, other: &Self) -> bool {
    self.kind == other.kind && self.message == other.message
    // We ignore the source field in equality comparison since we can't compare dyn StdError
  }
}

impl Eq for EffectError {}

impl Clone for EffectError {
  fn clone(&self) -> Self {
    Self {
      kind: self.kind.clone(),
      message: self.message.clone(),
      source: None, // We can't clone the source error, so we set it to None
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
  Io,
  Other,
}

impl EffectError {
  pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
    Self {
      kind,
      message: message.into(),
      source: None,
    }
  }

  pub fn with_source<E: StdError + Send + Sync + 'static>(
    kind: ErrorKind,
    message: impl Into<String>,
    source: E,
  ) -> Self {
    Self {
      kind,
      message: message.into(),
      source: Some(Box::new(source)),
    }
  }

  pub fn kind(&self) -> &ErrorKind {
    &self.kind
  }

  pub fn message(&self) -> &str {
    &self.message
  }
}

impl Display for EffectError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}: {}", self.kind, self.message)
  }
}

impl Display for ErrorKind {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      ErrorKind::Io => write!(f, "IO Error"),
      ErrorKind::Other => write!(f, "Other Error"),
    }
  }
}

impl StdError for EffectError {
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
    self.source.as_ref().map(|e| e.as_ref() as &dyn StdError)
  }
}

impl From<IoError> for EffectError {
  fn from(error: IoError) -> Self {
    Self::with_source(ErrorKind::Io, error.to_string(), error)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io;

  #[test]
  fn test_basic_error() {
    let error = EffectError::new(ErrorKind::Other, "test error");
    assert_eq!(error.kind(), &ErrorKind::Other);
    assert_eq!(error.message(), "test error");
    assert!(error.source().is_none());
  }

  #[test]
  fn test_error_with_source() {
    let io_error = io::Error::new(io::ErrorKind::Other, "io error");
    let error = EffectError::with_source(ErrorKind::Io, "test error", io_error);
    assert_eq!(error.kind(), &ErrorKind::Io);
    assert_eq!(error.message(), "test error");
    assert!(error.source().is_some());
  }

  #[test]
  fn test_error_clone() {
    let error = EffectError::new(ErrorKind::Other, "test error");
    let cloned = error.clone();
    assert_eq!(error.kind(), cloned.kind());
    assert_eq!(error.message(), cloned.message());
    assert!(cloned.source().is_none());
  }

  #[test]
  fn test_error_from_io() {
    let io_error = io::Error::new(io::ErrorKind::Other, "io error");
    let error: EffectError = io_error.into();
    assert_eq!(error.kind(), &ErrorKind::Io);
    assert!(error.source().is_some());
  }

  #[test]
  fn test_error_equality() {
    let error1 = EffectError::new(ErrorKind::Other, "test error");
    let error2 = EffectError::new(ErrorKind::Other, "test error");
    assert_eq!(error1, error2);

    let error3 = EffectError::new(ErrorKind::Io, "test error");
    assert_ne!(error1, error3);

    let error4 = EffectError::new(ErrorKind::Other, "different error");
    assert_ne!(error1, error4);
  }
}
