//! Integration tests for README examples
//!
//! These tests verify that all code examples in the README compile and run correctly.

use std::error::Error;
use streamweave_error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineError, StreamError,
};

/// Helper function to check if an error is transient (for custom handler example)
fn is_transient(_error: &dyn Error) -> bool {
  // Simplified check - in real code, this would examine error types
  false
}

/// Helper function to check if an error is a validation error (for custom handler example)
fn is_validation_error(_error: &dyn Error) -> bool {
  // Simplified check - in real code, this would examine error types
  false
}

/// Helper function to check if an error is critical (for error propagation example)
fn is_critical_error(_error: &dyn Error) -> bool {
  // Simplified check - in real code, this would examine error types
  false
}

#[test]
fn test_readme_basic_error_handling() {
  // Example: Basic Error Handling (lines 34-68)
  // This test verifies the basic error handling example compiles and works

  // Configure error strategy
  let strategy = ErrorStrategy::Skip; // Skip errors and continue

  // Create error context
  let context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: Some(42),
    component_name: "MyTransformer".to_string(),
    component_type: "Transformer".to_string(),
  };

  // Create component info
  let component = ComponentInfo::new("my-transformer".to_string(), "MyTransformer".to_string());

  // Create stream error
  let error = StreamError::new(
    Box::new(std::io::Error::other("Something went wrong")),
    context,
    component,
  );

  // Determine action based on strategy
  let action = match strategy {
    ErrorStrategy::Stop => ErrorAction::Stop,
    ErrorStrategy::Skip => ErrorAction::Skip,
    ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
    _ => ErrorAction::Stop,
  };

  assert_eq!(action, ErrorAction::Skip);
  assert_eq!(error.retries, 0);
}

#[test]
fn test_readme_stop_strategy() {
  // Example: Stop Strategy (lines 156-167)
  // This test verifies the stop strategy example

  let strategy = ErrorStrategy::<i32>::Stop;

  // When an error occurs, processing stops immediately
  // This ensures data integrity
  let error: StreamError<i32> = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext::default(),
    component: ComponentInfo::default(),
    retries: 0,
  };

  let action = match strategy {
    ErrorStrategy::Stop => ErrorAction::Stop,
    _ => ErrorAction::Skip,
  };

  assert_eq!(action, ErrorAction::Stop);
}

#[test]
fn test_readme_skip_strategy() {
  // Example: Skip Strategy (lines 169-180)
  // This test verifies the skip strategy example

  let strategy = ErrorStrategy::<i32>::Skip;

  // Invalid items are skipped, processing continues
  // Useful for data cleaning pipelines
  let error: StreamError<i32> = StreamError {
    source: Box::new(std::io::Error::other("validation error")),
    context: ErrorContext::default(),
    component: ComponentInfo::default(),
    retries: 0,
  };

  let action = match strategy {
    ErrorStrategy::Skip => ErrorAction::Skip,
    _ => ErrorAction::Stop,
  };

  assert_eq!(action, ErrorAction::Skip);
}

#[test]
fn test_readme_retry_strategy() {
  // Example: Retry Strategy (lines 182-193)
  // This test verifies the retry strategy example

  let strategy = ErrorStrategy::<i32>::Retry(3);

  // Retries up to 3 times before giving up
  // Useful for transient failures like network timeouts
  let mut error: StreamError<i32> = StreamError {
    source: Box::new(std::io::Error::other("network timeout")),
    context: ErrorContext::default(),
    component: ComponentInfo::default(),
    retries: 0,
  };

  // First retry attempt
  let action = match strategy {
    ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
    _ => ErrorAction::Stop,
  };
  assert_eq!(action, ErrorAction::Retry);

  // After 3 retries, should stop
  error.retries = 3;
  let action = match strategy {
    ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
    _ => ErrorAction::Stop,
  };
  assert_eq!(action, ErrorAction::Stop);
}

#[test]
fn test_readme_custom_error_handler() {
  // Example: Custom Error Handler (lines 195-216)
  // This test verifies the custom error handler example

  let strategy = ErrorStrategy::<i32>::new_custom(|error: &StreamError<i32>| {
    // Retry transient errors
    if error.retries < 3 && is_transient(error.source.as_ref()) {
      ErrorAction::Retry
    }
    // Skip validation errors
    else if is_validation_error(error.source.as_ref()) {
      ErrorAction::Skip
    }
    // Stop on critical errors
    else {
      ErrorAction::Stop
    }
  });

  let error: StreamError<i32> = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext::default(),
    component: ComponentInfo::default(),
    retries: 0,
  };

  // Test the custom handler
  if let ErrorStrategy::Custom(handler) = strategy {
    let action = handler(&error);
    // Since is_transient and is_validation_error return false, should stop
    assert_eq!(action, ErrorAction::Stop);
  } else {
    panic!("Expected Custom variant");
  }
}

#[test]
fn test_readme_creating_error_context() {
  // Example: Creating Error Context (lines 218-245)
  // This test verifies the error context creation example

  let problematic_item = 42;

  // Create error context
  let context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: Some(problematic_item),
    component_name: "DataValidator".to_string(),
    component_type: "Transformer".to_string(),
  };

  // Create component info
  let component = ComponentInfo::new("validator".to_string(), "DataValidator".to_string());

  // Create stream error
  let validation_error = std::io::Error::other("validation failed");
  let error = StreamError::new(
    Box::new(validation_error),
    context.clone(),
    component.clone(),
  );

  assert_eq!(error.context.item, Some(problematic_item));
  assert_eq!(error.context.component_name, "DataValidator");
  assert_eq!(error.component.name, "validator");
  assert_eq!(error.component.type_name, "DataValidator");
}

#[test]
fn test_readme_error_propagation_patterns() {
  // Example: Error Propagation Patterns (lines 247-270)
  // This test verifies the error propagation patterns example

  // Component-level error handling
  fn handle_component_error(error: &StreamError<i32>) -> ErrorAction {
    match error.retries {
      0..=2 => ErrorAction::Retry, // Retry first 3 attempts
      _ => ErrorAction::Skip,      // Skip after retries exhausted
    }
  }

  // Pipeline-level error handling
  fn handle_pipeline_error(error: &StreamError<i32>) -> ErrorAction {
    if is_critical_error(error.source.as_ref()) {
      ErrorAction::Stop // Stop on critical errors
    } else {
      ErrorAction::Skip // Skip non-critical errors
    }
  }

  // Test component-level handler
  let mut error: StreamError<i32> = StreamError {
    source: Box::new(std::io::Error::other("test")),
    context: ErrorContext::default(),
    component: ComponentInfo::default(),
    retries: 1,
  };
  assert_eq!(handle_component_error(&error), ErrorAction::Retry);

  error.retries = 3;
  assert_eq!(handle_component_error(&error), ErrorAction::Skip);

  // Test pipeline-level handler
  error.retries = 0;
  assert_eq!(handle_pipeline_error(&error), ErrorAction::Skip);
}

#[test]
fn test_readme_error_strategy_new_custom() {
  // Test the new_custom method from the API overview
  let strategy = ErrorStrategy::<i32>::new_custom(|error: &StreamError<i32>| {
    if error.retries < 2 {
      ErrorAction::Retry
    } else {
      ErrorAction::Stop
    }
  });

  let mut error: StreamError<i32> = StreamError {
    source: Box::new(std::io::Error::other("test")),
    context: ErrorContext::default(),
    component: ComponentInfo::default(),
    retries: 0,
  };

  if let ErrorStrategy::Custom(handler) = strategy {
    assert_eq!(handler(&error), ErrorAction::Retry);
    error.retries = 2;
    assert_eq!(handler(&error), ErrorAction::Stop);
  } else {
    panic!("Expected Custom variant");
  }
}

#[test]
fn test_readme_pipeline_error_creation() {
  // Test PipelineError creation (from API overview)
  let source = std::io::Error::other("pipeline error");
  let context: ErrorContext<i32> = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: Some(42),
    component_name: "TestComponent".to_string(),
    component_type: "Transformer".to_string(),
  };
  let component = ComponentInfo::new("test-component".to_string(), "TestComponent".to_string());

  let error: PipelineError<i32> = PipelineError::new(source, context.clone(), component.clone());

  assert_eq!(error.context(), &context);
  assert_eq!(error.component(), &component);
}

#[test]
fn test_readme_pipeline_error_from_stream_error() {
  // Test creating PipelineError from StreamError
  let stream_error: StreamError<i32> = StreamError {
    source: Box::new(std::io::Error::other("stream error")),
    context: ErrorContext::default(),
    component: ComponentInfo::default(),
    retries: 0,
  };

  let error: PipelineError<i32> = PipelineError::from_stream_error(stream_error.clone());

  assert_eq!(error.context(), &stream_error.context);
  assert_eq!(error.component(), &stream_error.component);
}
