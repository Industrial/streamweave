use std::error::Error;
use std::fmt;

/// Common initialization-related errors that can occur in producers, consumers, and transformers
#[derive(Debug, Clone, PartialEq)]
pub enum InitializationError {
  /// The component has not been initialized before use
  NotInitialized,
  /// The component has already been initialized
  AlreadyInitialized,
  /// The component has been consumed and cannot be reused
  AlreadyConsumed,
}

impl fmt::Display for InitializationError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      InitializationError::NotInitialized => write!(f, "Component not initialized"),
      InitializationError::AlreadyInitialized => write!(f, "Component already initialized"),
      InitializationError::AlreadyConsumed => write!(f, "Component already consumed"),
    }
  }
}

impl Error for InitializationError {}

/// Provides methods to inspect errors for initialization-related issues
pub trait InitializationErrorInspection {
  /// Returns true if the error is an initialization error
  fn is_initialization_error(&self) -> bool;

  /// Converts the error to an InitializationError if possible
  fn as_initialization_error(&self) -> Option<&InitializationError>;
}

/// Represents errors specific to one-time-use consumers
#[derive(Debug, Clone, PartialEq)]
pub enum ConsumptionError {
  /// The consumer has already been consumed and cannot be reused
  AlreadyConsumed,
}

impl fmt::Display for ConsumptionError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ConsumptionError::AlreadyConsumed => write!(f, "Consumer has already been consumed"),
    }
  }
}

impl Error for ConsumptionError {}

/// Trait for inspecting consumption-related errors
pub trait ConsumptionErrorInspection {
  /// Returns true if the error is a consumption error
  fn is_consumption_error(&self) -> bool;

  /// Converts the error to a ConsumptionError if possible
  fn as_consumption_error(&self) -> Option<&ConsumptionError>;
}

#[derive(Debug)]
pub enum TransformError {
  OperationFailed(Box<dyn Error + Send + Sync>),
  AllocationFailed,
  Custom(String),
}

impl fmt::Display for TransformError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::OperationFailed(e) => write!(f, "Transformation operation failed: {}", e),
      Self::AllocationFailed => write!(f, "Failed to allocate memory for transformation"),
      Self::Custom(msg) => write!(f, "{}", msg),
    }
  }
}

impl Error for TransformError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    match self {
      Self::OperationFailed(e) => Some(e.as_ref()),
      _ => None,
    }
  }
}
