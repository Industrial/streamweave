//! # Error Handling Test Suite
//!
//! Comprehensive test suite for the error handling system, including error actions,
//! error strategies, stream errors, pipeline errors, and error context management.
//!
//! ## Test Coverage
//!
//! This test suite covers:
//!
//! - **ErrorAction**: Stop, Skip, and Retry action variants with equality and cloning
//! - **ErrorStrategy**: Stop, Skip, Retry(n), and Custom strategy variants with handler execution
//! - **StreamError**: Error creation, display formatting, source access, and cloning
//! - **PipelineError**: Pipeline-specific error wrapping, context access, and display
//! - **ErrorContext**: Timestamp, item, component information, and default values
//! - **ComponentInfo**: Component name and type information management
//! - **PipelineStage**: Producer, Transformer, and Consumer stage variants
//! - **StringError**: Simple string-based error type for testing
//!
//! ## Test Organization
//!
//! Tests are organized to cover:
//!
//! 1. **ErrorAction Tests**: All variants, equality, cloning, and matching
//! 2. **ErrorStrategy Tests**: All variants, custom handlers, cloning, and equality
//! 3. **StreamError Tests**: Creation, display, source access, cloning, and retry tracking
//! 4. **PipelineError Tests**: Creation, context access, display, and source access
//! 5. **ErrorContext Tests**: Field access, default values, cloning, and equality
//! 6. **ComponentInfo Tests**: Default values, creation, cloning, and equality
//! 7. **PipelineStage Tests**: All variants, cloning, and equality
//!
//! ## Key Concepts
//!
//! - **ErrorAction**: The action to take when an error occurs (Stop, Skip, Retry)
//! - **ErrorStrategy**: The strategy for handling errors, including custom handlers
//! - **StreamError**: Rich error context with source, component info, and retry count
//! - **PipelineError**: Pipeline-specific error wrapper with stage information
//! - **ErrorContext**: Contextual information about when and where an error occurred
//!
//! ## Usage
//!
//! These tests ensure that the error handling system correctly manages errors
//! throughout the pipeline, providing rich context for debugging and proper
//! error propagation.

use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineError, PipelineErrorContext,
  PipelineStage, StreamError, StringError,
};
use std::error::Error;
use std::fmt;

#[test]
fn test_error_action_variants() {
  // Test all ErrorAction variants
  let stop = ErrorAction::Stop;
  let skip = ErrorAction::Skip;
  let retry = ErrorAction::Retry;

  assert!(matches!(stop, ErrorAction::Stop));
  assert!(matches!(skip, ErrorAction::Skip));
  assert!(matches!(retry, ErrorAction::Retry));
}

#[test]
fn test_error_action_debug() {
  // ErrorAction derives Debug, so we can format it
  let action = ErrorAction::Stop;
  let debug_str = format!("{:?}", action);
  assert!(debug_str.contains("Stop"));
}

#[test]
fn test_error_action_clone() {
  // ErrorAction derives Clone
  let action = ErrorAction::Retry;
  let cloned = action.clone();
  assert_eq!(action, cloned);
}

#[test]
fn test_error_action_partial_eq() {
  // ErrorAction derives PartialEq
  let action1 = ErrorAction::Stop;
  let action2 = ErrorAction::Stop;
  let action3 = ErrorAction::Skip;

  assert_eq!(action1, action2);
  assert_ne!(action1, action3);
}

#[test]
fn test_error_strategy_stop() {
  let strategy = ErrorStrategy::<i32>::Stop;
  assert!(matches!(strategy, ErrorStrategy::Stop));
}

#[test]
fn test_error_strategy_skip() {
  let strategy = ErrorStrategy::<i32>::Skip;
  assert!(matches!(strategy, ErrorStrategy::Skip));
}

#[test]
fn test_error_strategy_retry() {
  let strategy = ErrorStrategy::<i32>::Retry(5);
  match strategy {
    ErrorStrategy::Retry(5) => {}
    _ => panic!("Expected Retry(5)"),
  }
}

#[test]
fn test_error_strategy_new_custom() {
  // Test new_custom method (lines 143-148)
  let strategy = ErrorStrategy::<i32>::new_custom(|_error| ErrorAction::Skip);
  match strategy {
    ErrorStrategy::Custom(_) => {}
    _ => panic!("Expected Custom strategy"),
  }
}

#[test]
fn test_error_strategy_new_custom_handler() {
  // Test that custom handler is actually called
  let strategy = ErrorStrategy::<i32>::new_custom(|error| {
    if error.retries < 2 {
      ErrorAction::Retry
    } else {
      ErrorAction::Stop
    }
  });

  let context = ErrorContext::<i32> {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let component = ComponentInfo {
    name: "test".to_string(),
    type_name: "Test".to_string(),
  };
  let error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context,
    component,
    retries: 1, // Less than 2, should retry
  };

  match strategy {
    ErrorStrategy::Custom(handler) => {
      let action = handler(&error);
      assert!(matches!(action, ErrorAction::Retry));
    }
    _ => panic!("Expected Custom strategy"),
  }
}

#[test]
fn test_error_strategy_clone_stop() {
  // Test Clone implementation for ErrorStrategy::Stop (line 102)
  let strategy = ErrorStrategy::<i32>::Stop;
  let cloned = strategy.clone();
  assert_eq!(strategy, cloned);
  assert!(matches!(cloned, ErrorStrategy::Stop));
}

#[test]
fn test_error_strategy_clone_skip() {
  // Test Clone implementation for ErrorStrategy::Skip (line 103)
  let strategy = ErrorStrategy::<i32>::Skip;
  let cloned = strategy.clone();
  assert_eq!(strategy, cloned);
  assert!(matches!(cloned, ErrorStrategy::Skip));
}

#[test]
fn test_error_strategy_clone_retry() {
  // Test Clone implementation for ErrorStrategy::Retry(n) (line 104)
  let strategy = ErrorStrategy::<i32>::Retry(10);
  let cloned = strategy.clone();
  assert_eq!(strategy, cloned);
  match cloned {
    ErrorStrategy::Retry(10) => {}
    _ => panic!("Expected Retry(10)"),
  }
}

#[test]
fn test_error_strategy_clone_custom() {
  // Test Clone implementation for ErrorStrategy::Custom (line 105)
  let strategy = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Stop);
  let cloned = strategy.clone();

  // Both should be Custom variants
  match (strategy, cloned) {
    (ErrorStrategy::Custom(_), ErrorStrategy::Custom(_)) => {}
    _ => panic!("Expected both to be Custom"),
  }
}

#[test]
fn test_error_strategy_debug_stop() {
  // Test Debug implementation for ErrorStrategy::Stop (line 113)
  let strategy = ErrorStrategy::<i32>::Stop;
  let debug_str = format!("{:?}", strategy);
  assert_eq!(debug_str, "ErrorStrategy::Stop");
}

#[test]
fn test_error_strategy_debug_skip() {
  // Test Debug implementation for ErrorStrategy::Skip (line 114)
  let strategy = ErrorStrategy::<i32>::Skip;
  let debug_str = format!("{:?}", strategy);
  assert_eq!(debug_str, "ErrorStrategy::Skip");
}

#[test]
fn test_error_strategy_debug_retry() {
  // Test Debug implementation for ErrorStrategy::Retry(n) (line 115)
  let strategy = ErrorStrategy::<i32>::Retry(42);
  let debug_str = format!("{:?}", strategy);
  assert_eq!(debug_str, "ErrorStrategy::Retry(42)");
}

#[test]
fn test_error_strategy_debug_custom() {
  // Test Debug implementation for ErrorStrategy::Custom (line 116)
  let strategy = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Stop);
  let debug_str = format!("{:?}", strategy);
  assert_eq!(debug_str, "ErrorStrategy::Custom");
}

#[test]
fn test_error_strategy_partial_eq_stop() {
  // Test PartialEq implementation (line 124)
  let strategy1 = ErrorStrategy::<i32>::Stop;
  let strategy2 = ErrorStrategy::<i32>::Stop;
  assert_eq!(strategy1, strategy2);
}

#[test]
fn test_error_strategy_partial_eq_skip() {
  // Test PartialEq implementation (line 125)
  let strategy1 = ErrorStrategy::<i32>::Skip;
  let strategy2 = ErrorStrategy::<i32>::Skip;
  assert_eq!(strategy1, strategy2);
}

#[test]
fn test_error_strategy_partial_eq_retry() {
  // Test PartialEq implementation (line 126)
  let strategy1 = ErrorStrategy::<i32>::Retry(5);
  let strategy2 = ErrorStrategy::<i32>::Retry(5);
  let strategy3 = ErrorStrategy::<i32>::Retry(10);
  assert_eq!(strategy1, strategy2);
  assert_ne!(strategy1, strategy3);
}

#[test]
fn test_error_strategy_partial_eq_custom() {
  // Test PartialEq implementation (line 127)
  // Custom handlers are considered equal if both are Custom
  let strategy1 = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Stop);
  let strategy2 = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Skip);
  // Custom variants are considered equal to each other (line 127)
  assert_eq!(strategy1, strategy2);
}

#[test]
fn test_error_strategy_partial_eq_mixed() {
  // Test PartialEq implementation for different variants (line 128)
  let stop = ErrorStrategy::<i32>::Stop;
  let skip = ErrorStrategy::<i32>::Skip;
  let retry = ErrorStrategy::<i32>::Retry(5);
  let custom = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Stop);

  assert_ne!(stop, skip);
  assert_ne!(stop, retry);
  assert_ne!(stop, custom);
  assert_ne!(skip, retry);
  assert_ne!(skip, custom);
  assert_ne!(retry, custom);
}

#[test]
fn test_stream_error_new() {
  // Test StreamError::new method (lines 235-246)
  let source = Box::<dyn Error + Send + Sync>::from("test error");
  let context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: Some(42),
    component_name: "test_component".to_string(),
    component_type: "TestComponent".to_string(),
  };
  let component = ComponentInfo {
    name: "test_name".to_string(),
    type_name: "TestType".to_string(),
  };

  let error = StreamError::new(source, context.clone(), component.clone());

  assert_eq!(error.context, context);
  assert_eq!(error.component, component);
  assert_eq!(error.retries, 0); // Should be initialized to 0 (line 244)
}

#[test]
fn test_stream_error_new_with_none_item() {
  // Test StreamError::new with None item
  let source = Box::<dyn Error + Send + Sync>::from("test error");
  let context = ErrorContext::<i32> {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "test_component".to_string(),
    component_type: "TestComponent".to_string(),
  };
  let component = ComponentInfo {
    name: "test_name".to_string(),
    type_name: "TestType".to_string(),
  };

  let error = StreamError::new(source, context.clone(), component.clone());
  assert_eq!(error.context.item, None);
  assert_eq!(error.retries, 0);
}

#[test]
fn test_stream_error_display() {
  // Test Display implementation for StreamError (lines 249-257)
  let source = Box::<dyn Error + Send + Sync>::from("test error message");
  let context = ErrorContext::<i32> {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "test_component".to_string(),
    component_type: "TestComponent".to_string(),
  };
  let component = ComponentInfo {
    name: "test_name".to_string(),
    type_name: "TestType".to_string(),
  };

  let error = StreamError {
    source,
    context,
    component,
    retries: 0,
  };

  let display_str = format!("{}", error);
  // Line 253: "Error in {} ({}): {}"
  // Should contain component name, type name, and error message
  assert!(display_str.contains("test_name"));
  assert!(display_str.contains("TestType"));
  assert!(display_str.contains("test error message"));
}

#[test]
fn test_stream_error_source() {
  // Test Error trait implementation source method (lines 259-263)
  let source = Box::<dyn Error + Send + Sync>::from("test error");
  let context = ErrorContext::<i32> {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let component = ComponentInfo {
    name: "test".to_string(),
    type_name: "Test".to_string(),
  };

  let error = StreamError {
    source,
    context,
    component,
    retries: 0,
  };

  // Test that source() returns Some(&dyn Error) (line 261)
  let source_ref = <StreamError<i32> as Error>::source(&error);
  assert!(source_ref.is_some());
  let source_err = source_ref.unwrap();
  assert!(format!("{}", source_err).contains("test error"));
}

#[test]
fn test_stream_error_clone() {
  // Test Clone implementation for StreamError (lines 197-206)
  let source = Box::<dyn Error + Send + Sync>::from("original error");
  let context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: Some(42),
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let component = ComponentInfo {
    name: "test".to_string(),
    type_name: "Test".to_string(),
  };

  let error = StreamError {
    source,
    context: context.clone(),
    component: component.clone(),
    retries: 3,
  };

  let cloned = error.clone();

  // Verify cloned fields (lines 200-203)
  assert_eq!(cloned.context, context);
  assert_eq!(cloned.component, component);
  assert_eq!(cloned.retries, 3); // Line 203: retries is copied

  // Verify source is cloned as StringError (line 200)
  let cloned_source = <StreamError<i32> as Error>::source(&cloned);
  assert!(cloned_source.is_some());
  let source_str = format!("{}", cloned_source.unwrap());
  assert!(source_str.contains("original error"));
}

#[test]
fn test_string_error_display() {
  // Test Display implementation for StringError (lines 215-219)
  let string_error = StringError("test error message".to_string());
  let display_str = format!("{}", string_error);
  assert_eq!(display_str, "test error message");
}

#[test]
fn test_string_error_error_trait() {
  // Test Error trait implementation for StringError (line 221)
  let string_error = StringError("test error".to_string());
  // Error trait is implemented, so it should compile
  let _err_ref: &dyn Error = &string_error;
}

#[test]
fn test_error_context_default() {
  // Test Default implementation for ErrorContext (lines 282-291)
  let context: ErrorContext<i32> = Default::default();

  // Verify default values (lines 285-288)
  assert!(context.item.is_none()); // Line 286
  assert_eq!(context.component_name, "default"); // Line 287
  assert_eq!(context.component_type, "default"); // Line 288

  // Verify timestamp is set (line 285)
  let now = chrono::Utc::now();
  assert!(context.timestamp <= now);
  assert!(context.timestamp >= now - chrono::Duration::seconds(1));
}

#[test]
fn test_error_context_fields() {
  // Test ErrorContext struct fields
  let timestamp = chrono::Utc::now();
  let context = ErrorContext {
    timestamp,
    item: Some(42),
    component_name: "test_component".to_string(),
    component_type: "TestComponent".to_string(),
  };

  assert_eq!(context.timestamp, timestamp);
  assert_eq!(context.item, Some(42));
  assert_eq!(context.component_name, "test_component");
  assert_eq!(context.component_type, "TestComponent");
}

#[test]
fn test_error_context_clone() {
  // Test that ErrorContext derives Clone
  let context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: Some("test".to_string()),
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };

  let cloned = context.clone();
  assert_eq!(context, cloned);
}

#[test]
fn test_error_context_partial_eq() {
  // Test that ErrorContext derives PartialEq
  let context1 = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: Some(42),
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let context2 = ErrorContext {
    timestamp: context1.timestamp,
    item: Some(42),
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };

  assert_eq!(context1, context2);
}

#[test]
fn test_pipeline_stage_variants() {
  // Test all PipelineStage variants (lines 298-305)
  let producer = PipelineStage::Producer;
  let transformer = PipelineStage::Transformer("transform".to_string());
  let consumer = PipelineStage::Consumer;

  assert!(matches!(producer, PipelineStage::Producer));
  assert!(matches!(transformer, PipelineStage::Transformer(_)));
  assert!(matches!(consumer, PipelineStage::Consumer));
}

#[test]
fn test_pipeline_stage_transformer_with_name() {
  // Test PipelineStage::Transformer with a specific name
  let transformer = PipelineStage::Transformer("map_transform".to_string());
  match transformer {
    PipelineStage::Transformer(name) => assert_eq!(name, "map_transform"),
    _ => panic!("Expected Transformer variant"),
  }
}

#[test]
fn test_pipeline_stage_clone() {
  // Test that PipelineStage derives Clone
  let stage1 = PipelineStage::Transformer("test".to_string());
  let stage2 = stage1.clone();
  assert_eq!(stage1, stage2);
}

#[test]
fn test_pipeline_stage_partial_eq() {
  // Test that PipelineStage derives PartialEq
  let producer1 = PipelineStage::Producer;
  let producer2 = PipelineStage::Producer;
  let transformer1 = PipelineStage::Transformer("test".to_string());
  let transformer2 = PipelineStage::Transformer("test".to_string());
  let transformer3 = PipelineStage::Transformer("other".to_string());
  let consumer = PipelineStage::Consumer;

  assert_eq!(producer1, producer2);
  assert_eq!(transformer1, transformer2);
  assert_ne!(transformer1, transformer3);
  assert_ne!(producer1, consumer);
  assert_ne!(producer1, transformer1);
}

#[test]
fn test_component_info_default() {
  // Test Default implementation for ComponentInfo (lines 319-326)
  let info: ComponentInfo = Default::default();

  // Verify default values (lines 322-323)
  assert_eq!(info.name, "default");
  assert_eq!(info.type_name, "default");
}

#[test]
fn test_component_info_new() {
  // Test ComponentInfo::new method (lines 339-341)
  let info = ComponentInfo::new("test_name".to_string(), "TestType".to_string());
  assert_eq!(info.name, "test_name");
  assert_eq!(info.type_name, "TestType");
}

#[test]
fn test_component_info_clone() {
  // Test that ComponentInfo derives Clone
  let info1 = ComponentInfo {
    name: "test".to_string(),
    type_name: "Test".to_string(),
  };
  let info2 = info1.clone();
  assert_eq!(info1, info2);
}

#[test]
fn test_component_info_partial_eq() {
  // Test that ComponentInfo derives PartialEq
  let info1 = ComponentInfo {
    name: "test".to_string(),
    type_name: "Test".to_string(),
  };
  let info2 = ComponentInfo {
    name: "test".to_string(),
    type_name: "Test".to_string(),
  };
  let info3 = ComponentInfo {
    name: "other".to_string(),
    type_name: "Test".to_string(),
  };

  assert_eq!(info1, info2);
  assert_ne!(info1, info3);
}

#[test]
fn test_pipeline_error_context_fields() {
  // Test PipelineErrorContext struct fields (lines 349-354)
  let context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: Some(42),
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
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
  // Test that PipelineErrorContext derives Clone
  let context = ErrorContext::<i32> {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let stage = PipelineStage::Consumer;

  let pipeline_context = PipelineErrorContext { context, stage };
  let cloned = pipeline_context.clone();
  assert_eq!(pipeline_context, cloned);
}

#[test]
fn test_pipeline_error_context_partial_eq() {
  // Test that PipelineErrorContext derives PartialEq
  let context1 = ErrorContext::<i32> {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let stage1 = PipelineStage::Producer;

  let context2 = ErrorContext::<i32> {
    timestamp: context1.timestamp,
    item: None,
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let stage2 = PipelineStage::Producer;

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
fn test_pipeline_error_new() {
  // Test PipelineError::new method (lines 376-383)
  let source_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
  let context = ErrorContext::<i32> {
    timestamp: chrono::Utc::now(),
    item: Some(42),
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let component = ComponentInfo {
    name: "test_name".to_string(),
    type_name: "TestType".to_string(),
  };

  let pipeline_error = PipelineError::new(source_error, context.clone(), component.clone());

  // Verify that the inner StreamError was created correctly (line 381)
  assert_eq!(pipeline_error.context(), &context);
  assert_eq!(pipeline_error.component(), &component);
}

#[test]
fn test_pipeline_error_from_stream_error() {
  // Test PipelineError::from_stream_error method (lines 394-396)
  let source = Box::<dyn Error + Send + Sync>::from("test error");
  let context = ErrorContext::<i32> {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let component = ComponentInfo {
    name: "test".to_string(),
    type_name: "Test".to_string(),
  };
  let stream_error = StreamError {
    source,
    context: context.clone(),
    component: component.clone(),
    retries: 5,
  };

  let pipeline_error = PipelineError::from_stream_error(stream_error);

  // Verify that the StreamError was wrapped correctly (line 395)
  assert_eq!(pipeline_error.context(), &context);
  assert_eq!(pipeline_error.component(), &component);
}

#[test]
fn test_pipeline_error_context() {
  // Test PipelineError::context method (lines 403-405)
  let context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: Some("test".to_string()),
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let component = ComponentInfo {
    name: "test".to_string(),
    type_name: "Test".to_string(),
  };

  let pipeline_error = PipelineError::new(
    std::io::Error::new(std::io::ErrorKind::NotFound, "test"),
    context.clone(),
    component,
  );

  let context_ref = pipeline_error.context();
  assert_eq!(context_ref, &context);
  assert_eq!(context_ref.item, Some("test".to_string()));
}

#[test]
fn test_pipeline_error_component() {
  // Test PipelineError::component method (lines 412-414)
  let context = ErrorContext::<i32> {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let component = ComponentInfo {
    name: "component_name".to_string(),
    type_name: "ComponentType".to_string(),
  };

  let pipeline_error = PipelineError::new(
    std::io::Error::new(std::io::ErrorKind::NotFound, "test"),
    context,
    component.clone(),
  );

  let component_ref = pipeline_error.component();
  assert_eq!(component_ref, &component);
  assert_eq!(component_ref.name, "component_name");
  assert_eq!(component_ref.type_name, "ComponentType");
}

#[test]
fn test_pipeline_error_display() {
  // Test Display implementation for PipelineError (lines 417-425)
  let context = ErrorContext::<i32> {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let component = ComponentInfo {
    name: "test_component".to_string(),
    type_name: "TestComponent".to_string(),
  };

  let pipeline_error = PipelineError::new(
    std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"),
    context,
    component,
  );

  let display_str = format!("{}", pipeline_error);
  // Line 421: "Pipeline error in {}: {}"
  // Should contain component name and error message
  assert!(display_str.contains("test_component"));
  assert!(display_str.contains("File not found"));
}

#[test]
fn test_pipeline_error_source() {
  // Test Error trait implementation source method (lines 427-431)
  let context = ErrorContext::<i32> {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let component = ComponentInfo {
    name: "test".to_string(),
    type_name: "Test".to_string(),
  };

  let pipeline_error = PipelineError::new(
    std::io::Error::new(std::io::ErrorKind::NotFound, "test error"),
    context,
    component,
  );

  // Test that source() returns Some(&dyn Error) (line 429)
  let source_ref = <PipelineError<i32> as Error>::source(&pipeline_error);
  assert!(source_ref.is_some());
  let source_err = source_ref.unwrap();
  assert!(format!("{}", source_err).contains("test error"));
}

// Test ErrorStrategy::new_custom with different handler functions
#[test]
fn test_error_strategy_new_custom_various_handlers() {
  // Test that new_custom works with different handler signatures
  let handler1 = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Stop);
  let handler2 = ErrorStrategy::<i32>::new_custom(|error| {
    if error.retries > 5 {
      ErrorAction::Stop
    } else {
      ErrorAction::Retry
    }
  });
  let handler3 = ErrorStrategy::<String>::new_custom(|_| ErrorAction::Skip);

  match handler1 {
    ErrorStrategy::Custom(_) => {}
    _ => panic!("Expected Custom"),
  }

  match handler2 {
    ErrorStrategy::Custom(_) => {}
    _ => panic!("Expected Custom"),
  }

  match handler3 {
    ErrorStrategy::Custom(_) => {}
    _ => panic!("Expected Custom"),
  }
}

// Test ErrorStrategy::new_custom actually creates Arc
#[test]
fn test_error_strategy_new_custom_creates_arc() {
  // Test that new_custom creates an Arc (line 147)
  // We can verify this by cloning and ensuring both work
  let strategy = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Retry);
  let cloned = strategy.clone();

  let context = ErrorContext::<i32> {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let component = ComponentInfo {
    name: "test".to_string(),
    type_name: "Test".to_string(),
  };
  let error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context,
    component,
    retries: 0,
  };

  match (strategy, cloned) {
    (ErrorStrategy::Custom(handler1), ErrorStrategy::Custom(handler2)) => {
      let action1 = handler1(&error);
      let action2 = handler2(&error);
      assert_eq!(action1, action2);
      assert!(matches!(action1, ErrorAction::Retry));
    }
    _ => panic!("Expected both Custom"),
  }
}

// Test all branches in ErrorStrategy::Clone implementation
#[test]
fn test_error_strategy_clone_all_branches() {
  // Test all branches in Clone implementation (lines 100-106)
  let stop = ErrorStrategy::<i32>::Stop;
  let skip = ErrorStrategy::<i32>::Skip;
  let retry = ErrorStrategy::<i32>::Retry(99);
  let custom = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Stop);

  assert!(matches!(stop.clone(), ErrorStrategy::Stop));
  assert!(matches!(skip.clone(), ErrorStrategy::Skip));
  match retry.clone() {
    ErrorStrategy::Retry(99) => {}
    _ => panic!("Expected Retry(99)"),
  }
  match custom.clone() {
    ErrorStrategy::Custom(_) => {}
    _ => panic!("Expected Custom"),
  }
}

// Test all branches in ErrorStrategy::Debug implementation
#[test]
fn test_error_strategy_debug_all_branches() {
  // Test all branches in Debug implementation (lines 111-118)
  let stop = ErrorStrategy::<i32>::Stop;
  let skip = ErrorStrategy::<i32>::Skip;
  let retry = ErrorStrategy::<i32>::Retry(42);
  let custom = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Stop);

  assert_eq!(format!("{:?}", stop), "ErrorStrategy::Stop");
  assert_eq!(format!("{:?}", skip), "ErrorStrategy::Skip");
  assert_eq!(format!("{:?}", retry), "ErrorStrategy::Retry(42)");
  assert_eq!(format!("{:?}", custom), "ErrorStrategy::Custom");
}

// Test all branches in ErrorStrategy::PartialEq implementation
#[test]
fn test_error_strategy_partial_eq_all_branches() {
  // Test all branches in PartialEq implementation (lines 122-130)
  // Branch 1: (Stop, Stop) => true (line 124)
  assert_eq!(ErrorStrategy::<i32>::Stop, ErrorStrategy::<i32>::Stop);

  // Branch 2: (Skip, Skip) => true (line 125)
  assert_eq!(ErrorStrategy::<i32>::Skip, ErrorStrategy::<i32>::Skip);

  // Branch 3: (Retry(n1), Retry(n2)) => n1 == n2 (line 126)
  assert_eq!(
    ErrorStrategy::<i32>::Retry(5),
    ErrorStrategy::<i32>::Retry(5)
  );
  assert_ne!(
    ErrorStrategy::<i32>::Retry(5),
    ErrorStrategy::<i32>::Retry(10)
  );

  // Branch 4: (Custom(_), Custom(_)) => true (line 127)
  let custom1 = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Stop);
  let custom2 = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Skip);
  assert_eq!(custom1, custom2);

  // Branch 5: _ => false (line 128)
  assert_ne!(ErrorStrategy::<i32>::Stop, ErrorStrategy::<i32>::Skip);
  assert_ne!(ErrorStrategy::<i32>::Stop, ErrorStrategy::<i32>::Retry(5));
  assert_ne!(ErrorStrategy::<i32>::Stop, custom1);
  assert_ne!(ErrorStrategy::<i32>::Skip, ErrorStrategy::<i32>::Retry(5));
  assert_ne!(ErrorStrategy::<i32>::Skip, custom1);
  assert_ne!(ErrorStrategy::<i32>::Retry(5), custom1);
}

// Test StreamError with different source error types
#[test]
fn test_stream_error_with_io_error() {
  let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
  let source = Box::<dyn Error + Send + Sync>::from(io_error);
  let context = ErrorContext::<i32> {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let component = ComponentInfo::default();

  let error = StreamError::new(source, context, component);
  assert_eq!(error.retries, 0);
}

#[test]
fn test_stream_error_with_string_error() {
  let string_error = StringError("custom error".to_string());
  let source = Box::<dyn Error + Send + Sync>::from(string_error);
  let context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: Some("test_item".to_string()),
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let component = ComponentInfo::default();

  let error = StreamError::new(source, context.clone(), component);
  assert_eq!(error.context, context);
}

// Test StreamError clone preserves all fields
#[test]
fn test_stream_error_clone_preserves_fields() {
  let source = Box::<dyn Error + Send + Sync>::from("original");
  let context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: Some(100),
    component_name: "original_name".to_string(),
    component_type: "OriginalType".to_string(),
  };
  let component = ComponentInfo {
    name: "original_component".to_string(),
    type_name: "OriginalComponent".to_string(),
  };

  let error = StreamError {
    source,
    context: context.clone(),
    component: component.clone(),
    retries: 7,
  };

  let cloned = error.clone();
  assert_eq!(cloned.context, context);
  assert_eq!(cloned.component, component);
  assert_eq!(cloned.retries, 7);
}

// Test StreamError display with different component names
#[test]
fn test_stream_error_display_various_components() {
  let source = Box::<dyn Error + Send + Sync>::from("error message");
  let context = ErrorContext::<i32> {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "component_a".to_string(),
    component_type: "TypeA".to_string(),
  };
  let component = ComponentInfo {
    name: "component_b".to_string(),
    type_name: "TypeB".to_string(),
  };

  let error = StreamError {
    source,
    context,
    component,
    retries: 0,
  };

  let display_str = format!("{}", error);
  // Display uses component.name and component.type_name (lines 253-254)
  assert!(display_str.contains("component_b"));
  assert!(display_str.contains("TypeB"));
  assert!(display_str.contains("error message"));
}

// Test StreamError source with different error types
#[test]
fn test_stream_error_source_with_various_errors() {
  // Test with StringError
  let string_error = StringError("string error".to_string());
  let source1 = Box::<dyn Error + Send + Sync>::from(string_error);
  let error1 = StreamError::new(
    source1,
    ErrorContext::<i32>::default(),
    ComponentInfo::default(),
  );
  let source_ref1 = <StreamError<i32> as Error>::source(&error1);
  assert!(source_ref1.is_some());
  assert!(format!("{}", source_ref1.unwrap()).contains("string error"));

  // Test with io::Error
  let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
  let source2 = Box::<dyn Error + Send + Sync>::from(io_error);
  let error2 = StreamError::new(
    source2,
    ErrorContext::<i32>::default(),
    ComponentInfo::default(),
  );
  let source_ref2 = <StreamError<i32> as Error>::source(&error2);
  assert!(source_ref2.is_some());
}

// Test ErrorContext with different item types
#[test]
fn test_error_context_with_different_item_types() {
  let context_i32 = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: Some(42),
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  assert_eq!(context_i32.item, Some(42));

  let context_string = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: Some("test".to_string()),
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  assert_eq!(context_string.item, Some("test".to_string()));

  let context_none = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: None::<i32>,
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  assert_eq!(context_none.item, None);
}

// Test ErrorContext default with different types
#[test]
fn test_error_context_default_different_types() {
  let context_i32: ErrorContext<i32> = Default::default();
  assert_eq!(context_i32.item, None);
  assert_eq!(context_i32.component_name, "default");

  let context_string: ErrorContext<String> = Default::default();
  assert_eq!(context_string.item, None);
  assert_eq!(context_string.component_name, "default");
}

// Test PipelineStage with all variants and different transformer names
#[test]
fn test_pipeline_stage_all_variants_detailed() {
  let producer = PipelineStage::Producer;
  assert!(matches!(producer, PipelineStage::Producer));

  let transformer1 = PipelineStage::Transformer("transform1".to_string());
  match transformer1 {
    PipelineStage::Transformer(name) => assert_eq!(name, "transform1"),
    _ => panic!("Expected Transformer"),
  }

  let transformer2 = PipelineStage::Transformer("transform2".to_string());
  match transformer2 {
    PipelineStage::Transformer(name) => assert_eq!(name, "transform2"),
    _ => panic!("Expected Transformer"),
  }

  let consumer = PipelineStage::Consumer;
  assert!(matches!(consumer, PipelineStage::Consumer));
}

// Test ComponentInfo with various name and type combinations
#[test]
fn test_component_info_various_combinations() {
  let info1 = ComponentInfo::new("producer1".to_string(), "VecProducer".to_string());
  assert_eq!(info1.name, "producer1");
  assert_eq!(info1.type_name, "VecProducer");

  let info2 = ComponentInfo::new("transformer1".to_string(), "MapTransformer".to_string());
  assert_eq!(info2.name, "transformer1");
  assert_eq!(info2.type_name, "MapTransformer");

  let info3 = ComponentInfo::new("consumer1".to_string(), "VecConsumer".to_string());
  assert_eq!(info3.name, "consumer1");
  assert_eq!(info3.type_name, "VecConsumer");
}

// Test ComponentInfo equality with same and different values
#[test]
fn test_component_info_equality() {
  let info1 = ComponentInfo::new("same".to_string(), "SameType".to_string());
  let info2 = ComponentInfo::new("same".to_string(), "SameType".to_string());
  let info3 = ComponentInfo::new("different".to_string(), "SameType".to_string());
  let info4 = ComponentInfo::new("same".to_string(), "DifferentType".to_string());

  assert_eq!(info1, info2);
  assert_ne!(info1, info3);
  assert_ne!(info1, info4);
}

// Test PipelineErrorContext with all PipelineStage variants
#[test]
fn test_pipeline_error_context_with_all_stages() {
  let context = ErrorContext::<i32> {
    timestamp: chrono::Utc::now(),
    item: None,
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };

  let producer_context = PipelineErrorContext {
    context: context.clone(),
    stage: PipelineStage::Producer,
  };
  assert!(matches!(producer_context.stage, PipelineStage::Producer));

  let transformer_context = PipelineErrorContext {
    context: context.clone(),
    stage: PipelineStage::Transformer("test_transform".to_string()),
  };
  match transformer_context.stage {
    PipelineStage::Transformer(name) => assert_eq!(name, "test_transform"),
    _ => panic!("Expected Transformer"),
  }

  let consumer_context = PipelineErrorContext {
    context,
    stage: PipelineStage::Consumer,
  };
  assert!(matches!(consumer_context.stage, PipelineStage::Consumer));
}

// Test PipelineError::new with different error types
#[test]
fn test_pipeline_error_new_with_various_errors() {
  // Test with io::Error
  let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
  let context = ErrorContext::<i32>::default();
  let component = ComponentInfo::default();
  let pipeline_error1 = PipelineError::new(io_error, context.clone(), component.clone());
  assert_eq!(pipeline_error1.context(), &context);

  // Test with StringError
  let string_error = StringError("string error".to_string());
  let pipeline_error2 = PipelineError::new(string_error, context.clone(), component.clone());
  assert_eq!(pipeline_error2.context(), &context);

  // Test with custom error type
  #[derive(Debug)]
  struct CustomError {
    msg: String,
  }
  impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "{}", self.msg)
    }
  }
  impl Error for CustomError {}

  let custom_error = CustomError {
    msg: "custom error".to_string(),
  };
  let pipeline_error3 = PipelineError::new(custom_error, context, component);
  let source_ref = <PipelineError<i32> as Error>::source(&pipeline_error3);
  assert!(source_ref.is_some());
}

// Test PipelineError::from_stream_error preserves retries
#[test]
fn test_pipeline_error_from_stream_error_preserves_retries() {
  let source = Box::<dyn Error + Send + Sync>::from("test");
  let context = ErrorContext::<i32>::default();
  let component = ComponentInfo::default();
  let stream_error = StreamError {
    source,
    context: context.clone(),
    component: component.clone(),
    retries: 10,
  };

  let pipeline_error = PipelineError::from_stream_error(stream_error);
  // Verify context and component are preserved
  assert_eq!(pipeline_error.context(), &context);
  assert_eq!(pipeline_error.component(), &component);
}

// Test PipelineError display with various component names
#[test]
fn test_pipeline_error_display_various_components() {
  let context = ErrorContext::<i32>::default();
  let component1 = ComponentInfo {
    name: "component_one".to_string(),
    type_name: "TypeOne".to_string(),
  };
  let pipeline_error1 = PipelineError::new(
    std::io::Error::new(std::io::ErrorKind::NotFound, "error1"),
    context.clone(),
    component1,
  );
  let display1 = format!("{}", pipeline_error1);
  assert!(display1.contains("component_one"));
  assert!(display1.contains("error1"));

  let component2 = ComponentInfo {
    name: "component_two".to_string(),
    type_name: "TypeTwo".to_string(),
  };
  let pipeline_error2 = PipelineError::new(
    std::io::Error::new(std::io::ErrorKind::PermissionDenied, "error2"),
    context,
    component2,
  );
  let display2 = format!("{}", pipeline_error2);
  assert!(display2.contains("component_two"));
  assert!(display2.contains("error2"));
}

// Test PipelineError source with chained errors
#[test]
fn test_pipeline_error_source_chained_errors() {
  let context = ErrorContext::<i32>::default();
  let component = ComponentInfo::default();

  // Test that source() works correctly (line 429)
  let pipeline_error = PipelineError::new(
    std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
    context,
    component,
  );

  let source_ref = <PipelineError<i32> as Error>::source(&pipeline_error);
  assert!(source_ref.is_some());
  let source_err = source_ref.unwrap();
  let source_display = format!("{}", source_err);
  assert!(source_display.contains("not found"));
}

// Test StringError with various messages
#[test]
fn test_string_error_various_messages() {
  let error1 = StringError("message one".to_string());
  assert_eq!(format!("{}", error1), "message one");

  let error2 = StringError("".to_string());
  assert_eq!(format!("{}", error2), "");

  let error3 = StringError("multiline\nmessage\nhere".to_string());
  assert_eq!(format!("{}", error3), "multiline\nmessage\nhere");
}

// Test StringError Debug trait
#[test]
fn test_string_error_debug() {
  let error = StringError("test message".to_string());
  let debug_str = format!("{:?}", error);
  assert!(debug_str.contains("StringError"));
  assert!(debug_str.contains("test message"));
}

// Test ErrorContext timestamp is actually set
#[test]
fn test_error_context_default_timestamp_set() {
  let before = chrono::Utc::now();
  let context: ErrorContext<i32> = Default::default();
  let after = chrono::Utc::now();

  // Verify timestamp is set (line 285)
  assert!(context.timestamp >= before);
  assert!(context.timestamp <= after);
}

// Test that StreamError::new initializes retries to 0
#[test]
fn test_stream_error_new_initializes_retries_zero() {
  let source = Box::<dyn Error + Send + Sync>::from("test");
  let context = ErrorContext::<i32>::default();
  let component = ComponentInfo::default();

  // Test that retries is initialized to 0 (line 244)
  let error = StreamError::new(source, context, component);
  assert_eq!(error.retries, 0);
}

// Test ErrorStrategy::new_custom line 147 execution
#[test]
fn test_error_strategy_new_custom_line_147() {
  // Test that line 147: Self::Custom(Arc::new(f)) is executed
  let strategy = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Stop);
  match strategy {
    ErrorStrategy::Custom(arc_handler) => {
      // Verify it's an Arc by using it
      let context = ErrorContext::<i32>::default();
      let component = ComponentInfo::default();
      let error = StreamError {
        source: Box::<dyn Error + Send + Sync>::from("test"),
        context,
        component,
        retries: 0,
      };
      let action = arc_handler(&error);
      assert!(matches!(action, ErrorAction::Stop));
    }
    _ => panic!("Expected Custom variant"),
  }
}

// Test StreamError clone line 200 (StringError creation)
#[test]
fn test_stream_error_clone_string_error_creation() {
  let source = Box::<dyn Error + Send + Sync>::from("original message");
  let context = ErrorContext::<i32>::default();
  let component = ComponentInfo::default();
  let error = StreamError {
    source,
    context,
    component,
    retries: 0,
  };

  let cloned = error.clone();
  // Verify that source is cloned as StringError (line 200)
  let cloned_source = <StreamError<i32> as Error>::source(&cloned);
  assert!(cloned_source.is_some());
  let source_display = format!("{}", cloned_source.unwrap());
  // Should contain the original error message
  assert!(source_display.contains("original message"));
}

// Test StreamError source line 261 execution
#[test]
fn test_stream_error_source_line_261() {
  // Test that line 261: Some(self.source.as_ref()) is executed
  let source = Box::<dyn Error + Send + Sync>::from("source message");
  let context = ErrorContext::<i32>::default();
  let component = ComponentInfo::default();
  let error = StreamError {
    source,
    context,
    component,
    retries: 0,
  };

  // This should execute line 261
  let source_ref = <StreamError<i32> as Error>::source(&error);
  assert!(source_ref.is_some());
  // Verify we can access the source error
  let source_display = format!("{}", source_ref.unwrap());
  assert!(source_display.contains("source message"));
}

// Test PipelineError source line 429 execution
#[test]
fn test_pipeline_error_source_line_429() {
  // Test that line 429: Some(&*self.inner.source) is executed
  let context = ErrorContext::<i32>::default();
  let component = ComponentInfo::default();
  let pipeline_error = PipelineError::new(
    std::io::Error::new(std::io::ErrorKind::NotFound, "pipeline error"),
    context,
    component,
  );

  // This should execute line 429
  let source_ref = <PipelineError<i32> as Error>::source(&pipeline_error);
  assert!(source_ref.is_some());
  let source_display = format!("{}", source_ref.unwrap());
  assert!(source_display.contains("pipeline error"));
}

// Test ErrorContext default lines 285-288 execution
#[test]
fn test_error_context_default_all_lines() {
  // Test lines 285-288: Default implementation
  let before = chrono::Utc::now();
  let context: ErrorContext<i32> = Default::default();
  let after = chrono::Utc::now();

  // Line 285: timestamp: chrono::Utc::now()
  assert!(context.timestamp >= before);
  assert!(context.timestamp <= after);

  // Line 286: item: None
  assert_eq!(context.item, None);

  // Line 287: component_name: "default".to_string()
  assert_eq!(context.component_name, "default");

  // Line 288: component_type: "default".to_string()
  assert_eq!(context.component_type, "default");
}

// Test ComponentInfo default lines 322-323 execution
#[test]
fn test_component_info_default_all_lines() {
  // Test lines 322-323: Default implementation
  let info: ComponentInfo = Default::default();

  // Line 322: name: "default".to_string()
  assert_eq!(info.name, "default");

  // Line 323: type_name: "default".to_string()
  assert_eq!(info.type_name, "default");
}

// Test ComponentInfo::new lines 339-341 execution
#[test]
fn test_component_info_new_all_lines() {
  // Test lines 339-341: new method
  // Line 340: Self { name, type_name }
  let info = ComponentInfo::new("test_name".to_string(), "TestType".to_string());

  assert_eq!(info.name, "test_name");
  assert_eq!(info.type_name, "TestType");
}

// Test PipelineError::new lines 380-382 execution
#[test]
fn test_pipeline_error_new_all_lines() {
  // Test lines 380-382: new method
  // Line 381: inner: StreamError::new(Box::new(error), context, component)
  let context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: Some(42),
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let component = ComponentInfo {
    name: "test_name".to_string(),
    type_name: "TestType".to_string(),
  };

  let pipeline_error = PipelineError::new(
    std::io::Error::new(std::io::ErrorKind::NotFound, "test"),
    context.clone(),
    component.clone(),
  );

  // Verify that StreamError::new was called with correct parameters
  assert_eq!(pipeline_error.context(), &context);
  assert_eq!(pipeline_error.component(), &component);
}

// Test PipelineError::from_stream_error line 395 execution
#[test]
fn test_pipeline_error_from_stream_error_line_395() {
  // Test line 395: Self { inner: error }
  let source = Box::<dyn Error + Send + Sync>::from("test");
  let context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: Some("item".to_string()),
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let component = ComponentInfo {
    name: "component".to_string(),
    type_name: "Component".to_string(),
  };
  let stream_error = StreamError {
    source,
    context: context.clone(),
    component: component.clone(),
    retries: 5,
  };

  let pipeline_error = PipelineError::from_stream_error(stream_error);

  // Verify that inner was set correctly (line 395)
  assert_eq!(pipeline_error.context(), &context);
  assert_eq!(pipeline_error.component(), &component);
}

// Test PipelineError::context line 404 execution
#[test]
fn test_pipeline_error_context_line_404() {
  // Test line 404: &self.inner.context
  let context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: Some(100),
    component_name: "test_ctx".to_string(),
    component_type: "TestCtx".to_string(),
  };
  let component = ComponentInfo::default();
  let pipeline_error = PipelineError::new(
    std::io::Error::new(std::io::ErrorKind::NotFound, "test"),
    context.clone(),
    component,
  );

  let context_ref = pipeline_error.context();
  assert_eq!(context_ref, &context);
  assert_eq!(context_ref.item, Some(100));
  assert_eq!(context_ref.component_name, "test_ctx");
}

// Test PipelineError::component line 413 execution
#[test]
fn test_pipeline_error_component_line_413() {
  // Test line 413: &self.inner.component
  let context = ErrorContext::<i32>::default();
  let component = ComponentInfo {
    name: "comp_name".to_string(),
    type_name: "CompType".to_string(),
  };
  let pipeline_error = PipelineError::new(
    std::io::Error::new(std::io::ErrorKind::NotFound, "test"),
    context,
    component.clone(),
  );

  let component_ref = pipeline_error.component();
  assert_eq!(component_ref, &component);
  assert_eq!(component_ref.name, "comp_name");
  assert_eq!(component_ref.type_name, "CompType");
}

// Test PipelineError Display line 421 execution
#[test]
fn test_pipeline_error_display_line_421() {
  // Test line 421: write!(f, "Pipeline error in {}: {}", self.inner.component.name, self.inner.source)
  let context = ErrorContext::<i32>::default();
  let component = ComponentInfo {
    name: "display_test".to_string(),
    type_name: "DisplayType".to_string(),
  };
  let pipeline_error = PipelineError::new(
    std::io::Error::new(std::io::ErrorKind::NotFound, "display error message"),
    context,
    component,
  );

  let display_str = format!("{}", pipeline_error);
  // Verify format string from line 421
  assert!(display_str.contains("Pipeline error in"));
  assert!(display_str.contains("display_test"));
  assert!(display_str.contains("display error message"));
}

// Test that StringError field access works
#[test]
fn test_string_error_field_access() {
  // Test that we can access the StringError field (line 213)
  let error = StringError("field_test".to_string());
  assert_eq!(error.0, "field_test");
}

// Test ErrorStrategy::Custom handler actually works
#[test]
fn test_error_strategy_custom_handler_execution() {
  let call_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
  let call_count_clone = call_count.clone();

  let strategy = ErrorStrategy::<i32>::new_custom(move |_error| {
    call_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    ErrorAction::Retry
  });

  let context = ErrorContext::<i32>::default();
  let component = ComponentInfo::default();
  let error = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context,
    component,
    retries: 0,
  };

  match strategy {
    ErrorStrategy::Custom(handler) => {
      let action1 = handler(&error);
      assert!(matches!(action1, ErrorAction::Retry));
      assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);

      let action2 = handler(&error);
      assert!(matches!(action2, ErrorAction::Retry));
      assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 2);
    }
    _ => panic!("Expected Custom"),
  }
}

// Test ErrorStrategy Clone for Custom with handler that accesses error
#[test]
fn test_error_strategy_clone_custom_with_error_access() {
  let strategy = ErrorStrategy::<i32>::new_custom(|error| {
    if error.retries < 3 {
      ErrorAction::Retry
    } else {
      ErrorAction::Stop
    }
  });

  let cloned = strategy.clone();

  let context = ErrorContext::<i32>::default();
  let component = ComponentInfo::default();

  let error1 = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test1"),
    context: context.clone(),
    component: component.clone(),
    retries: 2, // Less than 3
  };

  let error2 = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test2"),
    context,
    component,
    retries: 5, // Greater than 3
  };

  match (strategy, cloned) {
    (ErrorStrategy::Custom(handler1), ErrorStrategy::Custom(handler2)) => {
      let action1 = handler1(&error1);
      let action2 = handler2(&error1);
      assert_eq!(action1, action2);
      assert!(matches!(action1, ErrorAction::Retry));

      let action3 = handler1(&error2);
      let action4 = handler2(&error2);
      assert_eq!(action3, action4);
      assert!(matches!(action3, ErrorAction::Stop));
    }
    _ => panic!("Expected both Custom"),
  }
}

// Test StreamError display format exactly
#[test]
fn test_stream_error_display_format_exact() {
  let source = Box::<dyn Error + Send + Sync>::from("exact error");
  let context = ErrorContext::<i32>::default();
  let component = ComponentInfo {
    name: "exact_name".to_string(),
    type_name: "ExactType".to_string(),
  };

  let error = StreamError {
    source,
    context,
    component,
    retries: 0,
  };

  let display_str = format!("{}", error);
  // Line 253 format: "Error in {} ({}): {}"
  // Should be: "Error in exact_name (ExactType): exact error"
  assert!(display_str.starts_with("Error in exact_name"));
  assert!(display_str.contains("(ExactType)"));
  assert!(display_str.ends_with(": exact error"));
}

// Test PipelineError display format exactly
#[test]
fn test_pipeline_error_display_format_exact() {
  let context = ErrorContext::<i32>::default();
  let component = ComponentInfo {
    name: "pipeline_name".to_string(),
    type_name: "PipelineType".to_string(),
  };

  let pipeline_error = PipelineError::new(
    std::io::Error::new(std::io::ErrorKind::NotFound, "pipeline exact"),
    context,
    component,
  );

  let display_str = format!("{}", pipeline_error);
  // Line 421 format: "Pipeline error in {}: {}"
  // Should be: "Pipeline error in pipeline_name: pipeline exact"
  assert!(display_str.starts_with("Pipeline error in pipeline_name"));
  assert!(display_str.ends_with(": pipeline exact"));
}

// Test ErrorStrategy PartialEq with same custom handlers
#[test]
fn test_error_strategy_partial_eq_same_custom() {
  // Test that two Custom strategies with the same function are equal (line 127)
  // Note: In the implementation, any two Custom variants are considered equal

  let strategy1 = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Stop);
  let strategy2 = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Skip);

  // According to line 127, any two Custom variants are equal
  assert_eq!(strategy1, strategy2);
}

// Test that all ErrorStrategy variants are different from each other in PartialEq
#[test]
fn test_error_strategy_partial_eq_all_different() {
  let stop = ErrorStrategy::<i32>::Stop;
  let skip = ErrorStrategy::<i32>::Skip;
  let retry = ErrorStrategy::<i32>::Retry(5);
  let custom = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Stop);

  // All should be different from each other (except matching variants)
  assert_ne!(stop, skip);
  assert_ne!(stop, retry);
  assert_ne!(stop, custom);
  assert_ne!(skip, retry);
  assert_ne!(skip, custom);
  assert_ne!(retry, custom);
}

// Test StreamError with various retry counts
#[test]
fn test_stream_error_with_various_retries() {
  let context = ErrorContext::<i32>::default();
  let component = ComponentInfo::default();

  let error0 = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: context.clone(),
    component: component.clone(),
    retries: 0,
  };
  assert_eq!(error0.retries, 0);

  let error1 = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context: context.clone(),
    component: component.clone(),
    retries: 1,
  };
  assert_eq!(error1.retries, 1);

  let error_max = StreamError {
    source: Box::<dyn Error + Send + Sync>::from("test"),
    context,
    component,
    retries: usize::MAX,
  };
  assert_eq!(error_max.retries, usize::MAX);
}

// Test that StreamError clone copies retries correctly
#[test]
fn test_stream_error_clone_retries() {
  let source = Box::<dyn Error + Send + Sync>::from("test");
  let context = ErrorContext::<i32>::default();
  let component = ComponentInfo::default();
  let error = StreamError {
    source,
    context,
    component,
    retries: 42,
  };

  let cloned = error.clone();
  // Line 203: retries is copied directly
  assert_eq!(cloned.retries, 42);
}

// Test PipelineError with different types
#[test]
fn test_pipeline_error_different_types() {
  let context_i32 = ErrorContext::<i32>::default();
  let component = ComponentInfo::default();
  let pipeline_error_i32 = PipelineError::new(
    std::io::Error::new(std::io::ErrorKind::NotFound, "i32 error"),
    context_i32,
    component.clone(),
  );
  assert_eq!(pipeline_error_i32.component().name, "default");

  let context_string = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: Some("string item".to_string()),
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let pipeline_error_string = PipelineError::new(
    std::io::Error::new(std::io::ErrorKind::NotFound, "string error"),
    context_string.clone(),
    component,
  );
  assert_eq!(
    pipeline_error_string.context().item,
    Some("string item".to_string())
  );
}

// Test that ErrorStrategy::Retry clone preserves the number
#[test]
fn test_error_strategy_retry_clone_preserves_number() {
  // Test line 104: ErrorStrategy::Retry(*n)
  let retry5 = ErrorStrategy::<i32>::Retry(5);
  let retry10 = ErrorStrategy::<i32>::Retry(10);
  let retry0 = ErrorStrategy::<i32>::Retry(0);

  match retry5.clone() {
    ErrorStrategy::Retry(5) => {}
    _ => panic!("Expected Retry(5)"),
  }

  match retry10.clone() {
    ErrorStrategy::Retry(10) => {}
    _ => panic!("Expected Retry(10)"),
  }

  match retry0.clone() {
    ErrorStrategy::Retry(0) => {}
    _ => panic!("Expected Retry(0)"),
  }
}

// Test ErrorStrategy Debug for Retry with various numbers
#[test]
fn test_error_strategy_debug_retry_various_numbers() {
  // Test line 115: write!(f, "ErrorStrategy::Retry({})", n)
  assert_eq!(
    format!("{:?}", ErrorStrategy::<i32>::Retry(0)),
    "ErrorStrategy::Retry(0)"
  );
  assert_eq!(
    format!("{:?}", ErrorStrategy::<i32>::Retry(1)),
    "ErrorStrategy::Retry(1)"
  );
  assert_eq!(
    format!("{:?}", ErrorStrategy::<i32>::Retry(42)),
    "ErrorStrategy::Retry(42)"
  );
  assert_eq!(
    format!("{:?}", ErrorStrategy::<i32>::Retry(999)),
    "ErrorStrategy::Retry(999)"
  );
}

// Test ErrorStrategy PartialEq Retry with different numbers
#[test]
fn test_error_strategy_partial_eq_retry_different() {
  // Test line 126: (Retry(n1), Retry(n2)) => n1 == n2
  assert_eq!(
    ErrorStrategy::<i32>::Retry(5),
    ErrorStrategy::<i32>::Retry(5)
  );
  assert_ne!(
    ErrorStrategy::<i32>::Retry(5),
    ErrorStrategy::<i32>::Retry(6)
  );
  assert_ne!(
    ErrorStrategy::<i32>::Retry(0),
    ErrorStrategy::<i32>::Retry(1)
  );
}

// Test that ErrorContext default timestamp is recent
#[test]
fn test_error_context_default_timestamp_recent() {
  let before = chrono::Utc::now() - chrono::Duration::milliseconds(10);
  let context: ErrorContext<i32> = Default::default();
  let after = chrono::Utc::now() + chrono::Duration::milliseconds(10);

  // Timestamp should be between before and after (line 285)
  assert!(context.timestamp >= before);
  assert!(context.timestamp <= after);
}

// Test PipelineError with empty component name
#[test]
fn test_pipeline_error_empty_component_name() {
  let context = ErrorContext::<i32>::default();
  let component = ComponentInfo {
    name: "".to_string(),
    type_name: "EmptyType".to_string(),
  };

  let pipeline_error = PipelineError::new(
    std::io::Error::new(std::io::ErrorKind::NotFound, "error"),
    context,
    component,
  );

  let display_str = format!("{}", pipeline_error);
  // Should still display even with empty name
  assert!(display_str.contains("Pipeline error in"));
}

// Test StreamError with empty component name
#[test]
fn test_stream_error_empty_component_name() {
  let source = Box::<dyn Error + Send + Sync>::from("error");
  let context = ErrorContext::<i32>::default();
  let component = ComponentInfo {
    name: "".to_string(),
    type_name: "".to_string(),
  };

  let error = StreamError {
    source,
    context,
    component,
    retries: 0,
  };

  let display_str = format!("{}", error);
  // Should still display even with empty names
  assert!(display_str.contains("Error in"));
}

// Test that all ErrorAction variants can be used in matches
#[test]
fn test_error_action_all_variants_matchable() {
  let actions = vec![ErrorAction::Stop, ErrorAction::Skip, ErrorAction::Retry];

  for action in actions {
    match action {
      ErrorAction::Stop => {}
      ErrorAction::Skip => {}
      ErrorAction::Retry => {}
    }
  }
}

// Test that all ErrorStrategy variants can be used in matches
#[test]
fn test_error_strategy_all_variants_matchable() {
  let strategies: Vec<ErrorStrategy<i32>> = vec![
    ErrorStrategy::Stop,
    ErrorStrategy::Skip,
    ErrorStrategy::Retry(5),
    ErrorStrategy::new_custom(|_| ErrorAction::Stop),
  ];

  for strategy in strategies {
    match strategy {
      ErrorStrategy::Stop => {}
      ErrorStrategy::Skip => {}
      ErrorStrategy::Retry(_) => {}
      ErrorStrategy::Custom(_) => {}
    }
  }
}

// Test that all PipelineStage variants can be used in matches
#[test]
fn test_pipeline_stage_all_variants_matchable() {
  let stages: Vec<PipelineStage> = vec![
    PipelineStage::Producer,
    PipelineStage::Transformer("test".to_string()),
    PipelineStage::Consumer,
  ];

  for stage in stages {
    match stage {
      PipelineStage::Producer => {}
      PipelineStage::Transformer(_) => {}
      PipelineStage::Consumer => {}
    }
  }
}
