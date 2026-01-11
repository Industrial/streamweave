//! # Error Handling System
//!
//! Comprehensive error handling system for StreamWeave pipelines and graphs,
//! providing rich error context, configurable error strategies, and proper
//! error propagation throughout the system.
//!
//! ## Overview
//!
//! The error handling system provides:
//!
//! - **Error Actions**: Stop, Skip, or Retry when errors occur
//! - **Error Strategies**: Configurable error handling policies (Stop, Skip, Retry, Custom)
//! - **Rich Error Context**: Timestamps, component info, item context, and retry tracking
//! - **Error Types**: StreamError for component-level errors, PipelineError for pipeline-level errors
//! - **Component Information**: Structured component identification for debugging
//!
//! ## Core Types
//!
//! - **ErrorAction**: The action to take when an error occurs (Stop, Skip, Retry)
//! - **ErrorStrategy**: The strategy for handling errors, including custom handlers
//! - **StreamError**: Rich error context with source, component info, and retry count
//! - **PipelineError**: Pipeline-specific error wrapper with stage information
//! - **ErrorContext**: Contextual information about when and where an error occurred
//! - **ComponentInfo**: Component name and type information for error reporting
//!
//! ## Example
//!
//! ```rust
//! use crate::error::{ErrorStrategy, ErrorAction, StreamError, ErrorContext, ComponentInfo};
//!
//! // Configure error strategy
//! let strategy = ErrorStrategy::Retry(3);
//!
//! // Create error context
//! let context = ErrorContext {
//!     timestamp: chrono::Utc::now(),
//!     item: Some(42),
//!     component_name: "my_transformer".to_string(),
//!     component_type: "MapTransformer".to_string(),
//! };
//!
//! // Create stream error
//! let error = StreamError::new(
//!     Box::new(std::io::Error::from(std::io::ErrorKind::NotFound)),
//!     context,
//!     ComponentInfo::new("transformer1".to_string(), "MapTransformer".to_string()),
//! );
//! ```
//!
//! ## Error Strategies
//!
//! - **Stop**: Immediately stop processing (default, ensures data integrity)
//! - **Skip**: Skip the problematic item and continue
//! - **Retry(n)**: Retry up to n times before stopping
//! - **Custom**: User-defined handler function for fine-grained control
//!
//! ## Usage
//!
//! Error handling is configured at the component level (Producer, Transformer, Consumer)
//! and can be overridden per-component. The system ensures errors are properly
//! tracked, logged, and handled according to the configured strategy.

use chrono;
use std::error::Error;
use std::fmt;
use std::sync::Arc;

/// Action to take when an error occurs in a pipeline component.
///
/// This enum is used by error strategies to determine how to handle errors
/// during stream processing.
///
/// # Example
///
/// ```rust
/// use crate::error::ErrorAction;
///
/// // Stop processing on error
/// let action = ErrorAction::Stop;
///
/// // Skip the item and continue
/// let action = ErrorAction::Skip;
///
/// // Retry the operation
/// let action = ErrorAction::Retry;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorAction {
  /// Stop processing immediately when an error occurs.
  ///
  /// This is the default behavior and ensures data integrity by preventing
  /// partial results after an error.
  Stop,
  /// Skip the item that caused the error and continue processing.
  ///
  /// Useful for non-critical errors where partial results are acceptable.
  Skip,
  /// Retry the operation that caused the error.
  ///
  /// Useful for transient failures that may succeed on retry.
  Retry,
}

// Type alias for the complex custom error handler function
type CustomErrorHandler<T> = Arc<dyn Fn(&StreamError<T>) -> ErrorAction + Send + Sync>;

/// Strategy for handling errors in pipeline components.
///
/// Error strategies determine how components respond to errors during
/// stream processing. Strategies can be set at the pipeline level or
/// overridden at the component level.
///
/// # Example
///
/// ```rust
/// use crate::error::ErrorStrategy;
///
/// // Stop on first error (default)
/// let strategy = ErrorStrategy::Stop;
///
/// // Skip errors and continue
/// let strategy = ErrorStrategy::Skip;
///
/// // Retry up to 3 times
/// let strategy = ErrorStrategy::Retry(3);
///
/// // Custom error handling
/// let strategy = ErrorStrategy::new_custom(|error| {
///     if error.retries < 2 {
///         ErrorAction::Retry
///     } else {
///         ErrorAction::Stop
///     }
/// });
/// ```
pub enum ErrorStrategy<T> {
  /// Stop processing immediately when an error occurs.
  ///
  /// This is the default strategy and ensures data integrity.
  Stop,
  /// Skip items that cause errors and continue processing.
  ///
  /// Useful for data cleaning scenarios where invalid records can be
  /// safely ignored.
  Skip,
  /// Retry failed operations up to the specified number of times.
  ///
  /// # Arguments
  ///
  /// * `usize` - Maximum number of retry attempts
  ///
  /// Useful for transient failures like network timeouts.
  Retry(usize),
  /// Custom error handling logic.
  ///
  /// Allows fine-grained control over error handling based on error
  /// context, type, or retry count.
  Custom(CustomErrorHandler<T>),
}

impl<T: std::fmt::Debug + Clone + Send + Sync> Clone for ErrorStrategy<T> {
  fn clone(&self) -> Self {
    match self {
      ErrorStrategy::Stop => ErrorStrategy::Stop,
      ErrorStrategy::Skip => ErrorStrategy::Skip,
      ErrorStrategy::Retry(n) => ErrorStrategy::Retry(*n),
      ErrorStrategy::Custom(handler) => ErrorStrategy::Custom(handler.clone()),
    }
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync> fmt::Debug for ErrorStrategy<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ErrorStrategy::Stop => write!(f, "ErrorStrategy::Stop"),
      ErrorStrategy::Skip => write!(f, "ErrorStrategy::Skip"),
      ErrorStrategy::Retry(n) => write!(f, "ErrorStrategy::Retry({})", n),
      ErrorStrategy::Custom(_) => write!(f, "ErrorStrategy::Custom"),
    }
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync> PartialEq for ErrorStrategy<T> {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (ErrorStrategy::Stop, ErrorStrategy::Stop) => true,
      (ErrorStrategy::Skip, ErrorStrategy::Skip) => true,
      (ErrorStrategy::Retry(n1), ErrorStrategy::Retry(n2)) => n1 == n2,
      (ErrorStrategy::Custom(_), ErrorStrategy::Custom(_)) => true,
      _ => false,
    }
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync> ErrorStrategy<T> {
  /// Creates a custom error handling strategy with a user-defined handler function.
  ///
  /// # Arguments
  ///
  /// * `f` - A function that takes a `StreamError` and returns an `ErrorAction`.
  ///
  /// # Returns
  ///
  /// A `Custom` error strategy that uses the provided handler function.
  pub fn new_custom<F>(f: F) -> Self
  where
    F: Fn(&StreamError<T>) -> ErrorAction + Send + Sync + 'static,
  {
    Self::Custom(Arc::new(f))
  }
}

/// Error that occurred during stream processing.
///
/// This error type provides rich context about where and when an error
/// occurred, making it easier to debug and handle errors appropriately.
///
/// # Fields
///
/// * `source` - The original error that occurred
/// * `context` - Context about when and where the error occurred
/// * `component` - Information about the component that encountered the error
/// * `retries` - Number of times this error has been retried
///
/// # Example
///
/// ```rust
/// use crate::error::{StreamError, ErrorContext, ComponentInfo};
/// use std::error::Error;
///
/// # fn example() {
/// let error = StreamError::new(
///     Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "File not found")),
///     ErrorContext {
///         timestamp: chrono::Utc::now(),
///         item: Some(42),
///         component_name: "FileProducer".to_string(),
///         component_type: "Producer".to_string(),
///     },
///     ComponentInfo {
///         name: "file-producer".to_string(),
///         type_name: "FileProducer".to_string(),
///     },
/// );
/// # }
/// ```
#[derive(Debug)]
pub struct StreamError<T> {
  /// The original error that occurred.
  pub source: Box<dyn Error + Send + Sync>,
  /// Context about when and where the error occurred.
  pub context: ErrorContext<T>,
  /// Information about the component that encountered the error.
  pub component: ComponentInfo,
  /// Number of times this error has been retried.
  pub retries: usize,
}

impl<T: std::fmt::Debug + Clone + Send + Sync> Clone for StreamError<T> {
  fn clone(&self) -> Self {
    Self {
      source: Box::new(StringError(self.source.to_string())),
      context: self.context.clone(),
      component: self.component.clone(),
      retries: self.retries,
    }
  }
}

/// A simple error type that wraps a string message.
///
/// This is useful for creating errors from string messages without
/// needing to implement a full error type.
#[derive(Debug)]
pub struct StringError(pub String);

impl std::fmt::Display for StringError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl std::error::Error for StringError {}

impl<T: std::fmt::Debug + Clone + Send + Sync> StreamError<T> {
  /// Creates a new `StreamError` with the given source error, context, and component information.
  ///
  /// # Arguments
  ///
  /// * `source` - The original error that occurred.
  /// * `context` - Context about when and where the error occurred.
  /// * `component` - Information about the component that encountered the error.
  ///
  /// # Returns
  ///
  /// A new `StreamError` with `retries` set to 0.
  pub fn new(
    source: Box<dyn Error + Send + Sync>,
    context: ErrorContext<T>,
    component: ComponentInfo,
  ) -> Self {
    Self {
      source,
      context,
      component,
      retries: 0,
    }
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync> fmt::Display for StreamError<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "Error in {} ({}): {}",
      self.component.name, self.component.type_name, self.source
    )
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync> Error for StreamError<T> {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    Some(self.source.as_ref())
  }
}

/// Context information about when and where an error occurred.
///
/// This struct provides detailed information about the circumstances
/// surrounding an error, including the timestamp, the item being processed
/// (if any), and the component that encountered the error.
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorContext<T> {
  /// The timestamp when the error occurred.
  pub timestamp: chrono::DateTime<chrono::Utc>,
  /// The item being processed when the error occurred, if available.
  pub item: Option<T>,
  /// The name of the component that encountered the error.
  pub component_name: String,
  /// The type of the component that encountered the error.
  pub component_type: String,
}

impl<T: std::fmt::Debug + Clone + Send + Sync> Default for ErrorContext<T> {
  fn default() -> Self {
    Self {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "default".to_string(),
      component_type: "default".to_string(),
    }
  }
}

/// Represents the stage in a pipeline where an error occurred.
///
/// This enum is used to identify which part of the pipeline
/// encountered an error, allowing for more targeted error handling.
#[derive(Debug, Clone, PartialEq)]
pub enum PipelineStage {
  /// Error occurred in a producer.
  Producer,
  /// Error occurred in a transformer, with the transformer name.
  Transformer(String),
  /// Error occurred in a consumer.
  Consumer,
}

/// Information about a pipeline component.
///
/// This struct provides identifying information about a component,
/// including its name and type, which is useful for logging and error reporting.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentInfo {
  /// The name of the component.
  pub name: String,
  /// The type name of the component.
  pub type_name: String,
}

impl Default for ComponentInfo {
  fn default() -> Self {
    Self {
      name: "default".to_string(),
      type_name: "default".to_string(),
    }
  }
}

impl ComponentInfo {
  /// Creates a new `ComponentInfo` with the given name and type name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the component.
  /// * `type_name` - The type name of the component.
  ///
  /// # Returns
  ///
  /// A new `ComponentInfo` instance.
  pub fn new(name: String, type_name: String) -> Self {
    Self { name, type_name }
  }
}

/// Extended error context that includes pipeline stage information.
///
/// This struct combines `ErrorContext` with `PipelineStage` to provide
/// comprehensive information about where an error occurred in the pipeline.
#[derive(Debug, Clone, PartialEq)]
pub struct PipelineErrorContext<T> {
  /// The base error context.
  pub context: ErrorContext<T>,
  /// The pipeline stage where the error occurred.
  pub stage: PipelineStage,
}

/// An error that occurred during pipeline execution.
///
/// This struct wraps a `StreamError` and provides pipeline-specific error information.
#[derive(Debug)]
pub struct PipelineError<T> {
  inner: StreamError<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync> PipelineError<T> {
  /// Creates a new `PipelineError` from an error, context, and component information.
  ///
  /// # Arguments
  ///
  /// * `error` - The underlying error that occurred.
  /// * `context` - The error context containing details about when and where the error occurred.
  /// * `component` - Information about the component where the error occurred.
  ///
  /// # Returns
  ///
  /// A new `PipelineError` instance.
  pub fn new<E>(error: E, context: ErrorContext<T>, component: ComponentInfo) -> Self
  where
    E: Error + Send + Sync + 'static,
  {
    Self {
      inner: StreamError::new(Box::new(error), context, component),
    }
  }

  /// Creates a new `PipelineError` from an existing `StreamError`.
  ///
  /// # Arguments
  ///
  /// * `error` - The `StreamError` to wrap.
  ///
  /// # Returns
  ///
  /// A new `PipelineError` instance.
  pub fn from_stream_error(error: StreamError<T>) -> Self {
    Self { inner: error }
  }

  /// Returns a reference to the error context.
  ///
  /// # Returns
  ///
  /// A reference to the `ErrorContext` associated with this error.
  pub fn context(&self) -> &ErrorContext<T> {
    &self.inner.context
  }

  /// Returns a reference to the component information.
  ///
  /// # Returns
  ///
  /// A reference to the `ComponentInfo` associated with this error.
  pub fn component(&self) -> &ComponentInfo {
    &self.inner.component
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync> std::fmt::Display for PipelineError<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "Pipeline error in {}: {}",
      self.inner.component.name, self.inner.source
    )
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync> Error for PipelineError<T> {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    Some(&*self.inner.source)
  }
}
