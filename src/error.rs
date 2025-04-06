use chrono::{DateTime, Utc};
use std::any::Any;
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

#[derive(Debug)]
pub struct StreamError {
  pub source: Box<dyn Error + Send + Sync>,
  pub context: ErrorContext,
  pub retries: usize,
  pub component: ComponentInfo,
}

#[derive(Debug)]
pub struct ErrorContext {
  pub timestamp: DateTime<Utc>,
  pub item: Option<Box<dyn Any + Send>>,
  pub stage: PipelineStage,
}

#[derive(Debug)]
pub enum PipelineStage {
  Producer,
  Transformer(String),
  Consumer,
}

#[derive(Debug)]
pub struct ComponentInfo {
  pub name: String,
  pub type_name: String,
}

#[derive(Debug, Clone)]
pub enum ErrorAction {
  Stop,
  Skip,
  Retry,
}

pub enum ErrorStrategy {
  Stop,
  Skip,
  Retry(usize),
  Custom(Box<dyn Fn(&StreamError) -> ErrorAction + Send + Sync>),
}

impl Clone for ErrorStrategy {
  fn clone(&self) -> Self {
    match self {
      ErrorStrategy::Stop => ErrorStrategy::Stop,
      ErrorStrategy::Skip => ErrorStrategy::Skip,
      ErrorStrategy::Retry(n) => ErrorStrategy::Retry(*n),
      ErrorStrategy::Custom(_) => ErrorStrategy::Stop, // Custom handlers can't be cloned
    }
  }
}

impl std::fmt::Debug for ErrorStrategy {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ErrorStrategy::Stop => write!(f, "ErrorStrategy::Stop"),
      ErrorStrategy::Skip => write!(f, "ErrorStrategy::Skip"),
      ErrorStrategy::Retry(n) => write!(f, "ErrorStrategy::Retry({})", n),
      ErrorStrategy::Custom(_) => write!(f, "ErrorStrategy::Custom"),
    }
  }
}

impl std::fmt::Display for ErrorStrategy {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ErrorStrategy::Stop => write!(f, "stop processing on error"),
      ErrorStrategy::Skip => write!(f, "skip item on error"),
      ErrorStrategy::Retry(n) => write!(f, "retry up to {} times on error", n),
      ErrorStrategy::Custom(_) => write!(f, "use custom error handler"),
    }
  }
}

#[derive(Debug)]
pub struct PipelineError {
  inner: StreamError,
}

impl PipelineError {
  pub fn new<E>(error: E, context: ErrorContext, component: ComponentInfo) -> Self
  where
    E: Error + Send + Sync + 'static,
  {
    Self {
      inner: StreamError {
        source: Box::new(error),
        context,
        retries: 0,
        component,
      },
    }
  }

  pub fn from_stream_error(error: StreamError) -> Self {
    Self { inner: error }
  }

  pub fn context(&self) -> &ErrorContext {
    &self.inner.context
  }

  pub fn component(&self) -> &ComponentInfo {
    &self.inner.component
  }
}

impl std::fmt::Display for PipelineError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "Pipeline error in {}: {}",
      self.inner.component.name, self.inner.source
    )
  }
}

impl Error for PipelineError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    Some(&*self.inner.source)
  }
}
