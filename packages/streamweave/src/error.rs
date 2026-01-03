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
pub enum ErrorStrategy<T: std::fmt::Debug + Clone + Send + Sync> {
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
pub struct StreamError<T: std::fmt::Debug + Clone + Send + Sync> {
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
pub struct ErrorContext<T: std::fmt::Debug + Clone + Send + Sync> {
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
pub struct PipelineErrorContext<T: std::fmt::Debug + Clone + Send + Sync> {
  /// The base error context.
  pub context: ErrorContext<T>,
  /// The pipeline stage where the error occurred.
  pub stage: PipelineStage,
}

/// An error that occurred during pipeline execution.
///
/// This struct wraps a `StreamError` and provides pipeline-specific error information.
#[derive(Debug)]
pub struct PipelineError<T: std::fmt::Debug + Clone + Send + Sync> {
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

#[cfg(test)]
mod tests {
  use super::*;
  use std::error::Error;

  #[test]
  fn test_error_action() {
    assert_eq!(ErrorAction::Stop, ErrorAction::Stop);
    assert_eq!(ErrorAction::Skip, ErrorAction::Skip);
    assert_eq!(ErrorAction::Retry, ErrorAction::Retry);
    assert_ne!(ErrorAction::Stop, ErrorAction::Skip);
    assert_ne!(ErrorAction::Skip, ErrorAction::Retry);
    assert_ne!(ErrorAction::Retry, ErrorAction::Stop);
  }

  #[test]
  fn test_error_strategy_clone() {
    let strategy = ErrorStrategy::<i32>::Stop;
    assert_eq!(strategy.clone(), ErrorStrategy::Stop);

    let strategy = ErrorStrategy::<i32>::Skip;
    assert_eq!(strategy.clone(), ErrorStrategy::Skip);

    let strategy = ErrorStrategy::<i32>::Retry(3);
    assert_eq!(strategy.clone(), ErrorStrategy::Retry(3));

    let strategy = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Skip);
    let cloned = strategy.clone();
    let _error: StreamError<i32> = StreamError {
      source: Box::new(StringError("test".to_string())),
      context: ErrorContext::default(),
      component: ComponentInfo::default(),
      retries: 0,
    };
    assert_eq!(strategy, cloned);
  }

  #[test]
  fn test_error_strategy_debug() {
    assert_eq!(
      format!("{:?}", ErrorStrategy::<i32>::Stop),
      "ErrorStrategy::Stop"
    );
    assert_eq!(
      format!("{:?}", ErrorStrategy::<i32>::Skip),
      "ErrorStrategy::Skip"
    );
    assert_eq!(
      format!("{:?}", ErrorStrategy::<i32>::Retry(3)),
      "ErrorStrategy::Retry(3)"
    );
    assert_eq!(
      format!(
        "{:?}",
        ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Skip)
      ),
      "ErrorStrategy::Custom"
    );
  }

  #[test]
  fn test_error_strategy_partial_eq() {
    assert_eq!(ErrorStrategy::<i32>::Stop, ErrorStrategy::Stop);
    assert_eq!(ErrorStrategy::<i32>::Skip, ErrorStrategy::Skip);
    assert_eq!(ErrorStrategy::<i32>::Retry(3), ErrorStrategy::Retry(3));
    assert_ne!(ErrorStrategy::<i32>::Stop, ErrorStrategy::Skip);
    assert_ne!(ErrorStrategy::<i32>::Skip, ErrorStrategy::Retry(3));
    assert_ne!(ErrorStrategy::<i32>::Retry(3), ErrorStrategy::Stop);
  }

  #[test]
  fn test_error_strategy_new_custom() {
    let strategy = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Skip);
    let error: StreamError<i32> = StreamError {
      source: Box::new(StringError("test".to_string())),
      context: ErrorContext::default(),
      component: ComponentInfo::default(),
      retries: 0,
    };
    if let ErrorStrategy::Custom(handler) = strategy {
      assert_eq!(handler(&error), ErrorAction::Skip);
    } else {
      panic!("Expected Custom variant");
    }
  }

  #[test]
  fn test_stream_error_clone() {
    let error: StreamError<i32> = StreamError {
      source: Box::new(StringError("test".to_string())),
      context: ErrorContext::default(),
      component: ComponentInfo::default(),
      retries: 0,
    };
    let cloned = error.clone();
    assert_eq!(error.source.to_string(), cloned.source.to_string());
    assert_eq!(error.context, cloned.context);
    assert_eq!(error.component, cloned.component);
    assert_eq!(error.retries, cloned.retries);
  }

  #[test]
  fn test_stream_error_display() {
    let error: StreamError<i32> = StreamError {
      source: Box::new(StringError("test error".to_string())),
      context: ErrorContext::default(),
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "TestComponent".to_string(),
      },
      retries: 0,
    };
    assert_eq!(
      error.to_string(),
      "Error in test (TestComponent): test error"
    );
  }

  #[test]
  fn test_stream_error_error() {
    let source = StringError("test error".to_string());
    let error: StreamError<i32> = StreamError {
      source: Box::new(source),
      context: ErrorContext::default(),
      component: ComponentInfo::default(),
      retries: 0,
    };
    assert_eq!(error.source().unwrap().to_string(), "test error");
  }

  #[test]
  fn test_stream_error_new() {
    let source = StringError("test error".to_string());
    let context: ErrorContext<i32> = ErrorContext::default();
    let component = ComponentInfo::default();
    let error = StreamError::new(Box::new(source), context.clone(), component.clone());
    assert_eq!(error.source.to_string(), "test error");
    assert_eq!(error.context, context);
    assert_eq!(error.component, component);
    assert_eq!(error.retries, 0);
  }

  #[test]
  fn test_error_context_default() {
    let context = ErrorContext::<i32>::default();
    assert!(context.timestamp <= chrono::Utc::now());
    assert_eq!(context.item, None);
    assert_eq!(context.component_name, "default");
    assert_eq!(context.component_type, "default");
  }

  #[test]
  fn test_error_context_clone() {
    let context = ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(42),
      component_name: "test".to_string(),
      component_type: "TestComponent".to_string(),
    };
    let cloned = context.clone();
    assert_eq!(context.timestamp, cloned.timestamp);
    assert_eq!(context.item, cloned.item);
    assert_eq!(context.component_name, cloned.component_name);
    assert_eq!(context.component_type, cloned.component_type);
  }

  #[test]
  fn test_error_context_partial_eq() {
    let context1 = ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(42),
      component_name: "test".to_string(),
      component_type: "TestComponent".to_string(),
    };
    let context2 = ErrorContext {
      timestamp: context1.timestamp,
      item: Some(42),
      component_name: "test".to_string(),
      component_type: "TestComponent".to_string(),
    };
    assert_eq!(context1, context2);
  }

  #[test]
  fn test_component_info_default() {
    let info = ComponentInfo::default();
    assert_eq!(info.name, "default");
    assert_eq!(info.type_name, "default");
  }

  #[test]
  fn test_component_info_clone() {
    let info = ComponentInfo {
      name: "test".to_string(),
      type_name: "TestComponent".to_string(),
    };
    let cloned = info.clone();
    assert_eq!(info.name, cloned.name);
    assert_eq!(info.type_name, cloned.type_name);
  }

  #[test]
  fn test_component_info_partial_eq() {
    let info1 = ComponentInfo {
      name: "test".to_string(),
      type_name: "TestComponent".to_string(),
    };
    let info2 = ComponentInfo {
      name: "test".to_string(),
      type_name: "TestComponent".to_string(),
    };
    assert_eq!(info1, info2);
  }

  #[test]
  fn test_component_info_new() {
    let info = ComponentInfo::new("test".to_string(), "TestComponent".to_string());
    assert_eq!(info.name, "test");
    assert_eq!(info.type_name, "TestComponent");
  }

  #[test]
  fn test_pipeline_error_new() {
    let source = StringError("test error".to_string());
    let context: ErrorContext<i32> = ErrorContext::default();
    let component = ComponentInfo::default();
    let error: PipelineError<i32> = PipelineError::new(source, context.clone(), component.clone());
    assert_eq!(error.context(), &context);
    assert_eq!(error.component(), &component);
  }

  #[test]
  fn test_pipeline_error_from_stream_error() {
    let stream_error: StreamError<i32> = StreamError {
      source: Box::new(StringError("test error".to_string())),
      context: ErrorContext::default(),
      component: ComponentInfo::default(),
      retries: 0,
    };
    let error: PipelineError<i32> = PipelineError::from_stream_error(stream_error.clone());
    assert_eq!(error.context(), &stream_error.context);
    assert_eq!(error.component(), &stream_error.component);
  }

  #[test]
  fn test_pipeline_error_display() {
    let error: PipelineError<i32> = PipelineError::new(
      StringError("test error".to_string()),
      ErrorContext::default(),
      ComponentInfo {
        name: "test".to_string(),
        type_name: "TestComponent".to_string(),
      },
    );
    assert_eq!(error.to_string(), "Pipeline error in test: test error");
  }

  #[test]
  fn test_pipeline_error_error() {
    let source = StringError("test error".to_string());
    let error: PipelineError<i32> =
      PipelineError::new(source, ErrorContext::default(), ComponentInfo::default());
    assert_eq!(error.source().unwrap().to_string(), "test error");
  }

  #[test]
  fn test_pipeline_stage_producer() {
    let stage = PipelineStage::Producer;
    assert_eq!(stage, PipelineStage::Producer);
    assert_ne!(stage, PipelineStage::Consumer);
  }

  #[test]
  fn test_pipeline_stage_transformer() {
    let stage1 = PipelineStage::Transformer("test".to_string());
    let stage2 = PipelineStage::Transformer("test".to_string());
    let stage3 = PipelineStage::Transformer("other".to_string());
    assert_eq!(stage1, stage2);
    assert_ne!(stage1, stage3);
    assert_ne!(stage1, PipelineStage::Producer);
    assert_ne!(stage1, PipelineStage::Consumer);
  }

  #[test]
  fn test_pipeline_stage_consumer() {
    let stage = PipelineStage::Consumer;
    assert_eq!(stage, PipelineStage::Consumer);
    assert_ne!(stage, PipelineStage::Producer);
  }

  #[test]
  fn test_pipeline_stage_clone() {
    let stage1 = PipelineStage::Transformer("test".to_string());
    let stage2 = stage1.clone();
    assert_eq!(stage1, stage2);
  }

  #[test]
  fn test_pipeline_stage_debug() {
    let stage = PipelineStage::Transformer("test".to_string());
    let debug_str = format!("{:?}", stage);
    assert!(debug_str.contains("Transformer"));
    assert!(debug_str.contains("test"));
  }

  #[test]
  fn test_pipeline_error_context_new() {
    let context = ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(42),
      component_name: "test".to_string(),
      component_type: "TestComponent".to_string(),
    };
    let stage = PipelineStage::Producer;
    let pipeline_context = PipelineErrorContext {
      context: context.clone(),
      stage: stage.clone(),
    };
    assert_eq!(pipeline_context.context, context);
    assert_eq!(pipeline_context.stage, stage);
  }

  #[test]
  fn test_pipeline_error_context_clone() {
    let context = ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(42),
      component_name: "test".to_string(),
      component_type: "TestComponent".to_string(),
    };
    let stage = PipelineStage::Transformer("test".to_string());
    let pipeline_context = PipelineErrorContext {
      context: context.clone(),
      stage: stage.clone(),
    };
    let cloned = pipeline_context.clone();
    assert_eq!(pipeline_context, cloned);
  }

  #[test]
  fn test_pipeline_error_context_partial_eq() {
    let context1 = ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(42),
      component_name: "test".to_string(),
      component_type: "TestComponent".to_string(),
    };
    let context2 = ErrorContext {
      timestamp: context1.timestamp,
      item: Some(42),
      component_name: "test".to_string(),
      component_type: "TestComponent".to_string(),
    };
    let stage1 = PipelineStage::Consumer;
    let stage2 = PipelineStage::Consumer;
    let pipeline_context1 = PipelineErrorContext {
      context: context1,
      stage: stage1,
    };
    let pipeline_context2 = PipelineErrorContext {
      context: context2,
      stage: stage2,
    };
    assert_eq!(pipeline_context1, pipeline_context2);
  }

  #[test]
  fn test_pipeline_error_context_debug() {
    let context = ErrorContext::<()>::default();
    let stage = PipelineStage::Producer;
    let pipeline_context = PipelineErrorContext { context, stage };
    let debug_str = format!("{:?}", pipeline_context);
    assert!(debug_str.contains("PipelineErrorContext"));
  }

  #[test]
  fn test_error_strategy_custom_handler_with_retries() {
    let strategy = ErrorStrategy::<i32>::new_custom(|error: &StreamError<i32>| {
      if error.retries < 2 {
        ErrorAction::Retry
      } else if error.retries < 5 {
        ErrorAction::Skip
      } else {
        ErrorAction::Stop
      }
    });

    let mut error = StreamError {
      source: Box::new(StringError("test".to_string())),
      context: ErrorContext::default(),
      component: ComponentInfo::default(),
      retries: 0,
    };

    if let ErrorStrategy::Custom(handler) = strategy {
      assert_eq!(handler(&error), ErrorAction::Retry);
      error.retries = 2;
      assert_eq!(handler(&error), ErrorAction::Skip);
      error.retries = 5;
      assert_eq!(handler(&error), ErrorAction::Stop);
    } else {
      panic!("Expected Custom variant");
    }
  }

  #[test]
  fn test_error_strategy_partial_eq_custom() {
    let strategy1 = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Skip);
    let strategy2 = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Stop);
    // Custom handlers are considered equal regardless of implementation
    assert_eq!(strategy1, strategy2);
  }

  #[test]
  fn test_stream_error_with_retries() {
    let mut error = StreamError::<()>::new(
      Box::new(StringError("test".to_string())),
      ErrorContext::<()>::default(),
      ComponentInfo::default(),
    );
    assert_eq!(error.retries, 0);
    error.retries = 5;
    assert_eq!(error.retries, 5);
  }

  #[test]
  fn test_string_error_display() {
    let error = StringError("test error message".to_string());
    assert_eq!(error.to_string(), "test error message");
  }

  #[test]
  fn test_string_error_error_trait() {
    let error = StringError("test".to_string());
    // Test that it implements Error trait
    let _: &dyn std::error::Error = &error;
  }

  #[test]
  fn test_error_context_with_item() {
    let context = ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some("test item".to_string()),
      component_name: "test".to_string(),
      component_type: "Test".to_string(),
    };
    assert_eq!(context.item, Some("test item".to_string()));
  }

  #[test]
  fn test_error_context_without_item() {
    let context = ErrorContext::<()> {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "Test".to_string(),
    };
    assert_eq!(context.item, None);
  }

  #[test]
  fn test_error_context_partial_eq_different_timestamps() {
    let timestamp1 = chrono::Utc::now();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let timestamp2 = chrono::Utc::now();

    let context1 = ErrorContext {
      timestamp: timestamp1,
      item: Some(42),
      component_name: "test".to_string(),
      component_type: "Test".to_string(),
    };
    let context2 = ErrorContext {
      timestamp: timestamp2,
      item: Some(42),
      component_name: "test".to_string(),
      component_type: "Test".to_string(),
    };
    // Timestamps differ, so contexts are not equal
    assert_ne!(context1, context2);
    // But if we set same timestamp, they should be equal
    let context3 = ErrorContext {
      timestamp: timestamp1,
      item: Some(42),
      component_name: "test".to_string(),
      component_type: "Test".to_string(),
    };
    assert_eq!(context1, context3);
  }
}
