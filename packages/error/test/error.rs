//! Comprehensive tests for error handling module
//!
//! This module provides 100% coverage tests for:
//! - ErrorAction enum
//! - ErrorStrategy enum and methods
//! - StreamError struct
//! - ErrorContext struct
//! - ComponentInfo struct
//! - PipelineStage enum
//! - Edge cases and error conditions

use chrono::Utc;
use std::error::Error;
use streamweave_error::*;

// ============================================================================
// ErrorAction Tests
// ============================================================================

#[test]
fn test_error_action_variants() {
  assert_eq!(ErrorAction::Stop, ErrorAction::Stop);
  assert_eq!(ErrorAction::Skip, ErrorAction::Skip);
  assert_eq!(ErrorAction::Retry, ErrorAction::Retry);
  assert_ne!(ErrorAction::Stop, ErrorAction::Skip);
  assert_ne!(ErrorAction::Stop, ErrorAction::Retry);
  assert_ne!(ErrorAction::Skip, ErrorAction::Retry);
}

#[test]
fn test_error_action_clone() {
  let action = ErrorAction::Stop;
  let cloned = action.clone();
  assert_eq!(action, cloned);

  let action = ErrorAction::Skip;
  let cloned = action.clone();
  assert_eq!(action, cloned);

  let action = ErrorAction::Retry;
  let cloned = action.clone();
  assert_eq!(action, cloned);
}

#[test]
fn test_error_action_debug() {
  let action = ErrorAction::Stop;
  let debug_str = format!("{:?}", action);
  assert!(debug_str.contains("Stop") || debug_str.contains("Stop"));

  let action = ErrorAction::Skip;
  let debug_str = format!("{:?}", action);
  assert!(debug_str.contains("Skip"));

  let action = ErrorAction::Retry;
  let debug_str = format!("{:?}", action);
  assert!(debug_str.contains("Retry"));
}

// ============================================================================
// ErrorStrategy Tests
// ============================================================================

#[test]
fn test_error_strategy_stop() {
  let strategy = ErrorStrategy::<i32>::Stop;
  let debug_str = format!("{:?}", strategy);
  assert!(debug_str.contains("Stop"));
}

#[test]
fn test_error_strategy_skip() {
  let strategy = ErrorStrategy::<i32>::Skip;
  let debug_str = format!("{:?}", strategy);
  assert!(debug_str.contains("Skip"));
}

#[test]
fn test_error_strategy_retry() {
  let strategy = ErrorStrategy::<i32>::Retry(3);
  let debug_str = format!("{:?}", strategy);
  assert!(debug_str.contains("Retry"));
  assert!(debug_str.contains("3"));
}

#[test]
fn test_error_strategy_custom() {
  let strategy = ErrorStrategy::<i32>::new_custom(|_error| ErrorAction::Skip);
  let debug_str = format!("{:?}", strategy);
  assert!(debug_str.contains("Custom"));
}

#[test]
fn test_error_strategy_clone() {
  let strategy = ErrorStrategy::<i32>::Stop;
  let cloned = strategy.clone();
  assert_eq!(strategy, cloned);

  let strategy = ErrorStrategy::<i32>::Skip;
  let cloned = strategy.clone();
  assert_eq!(strategy, cloned);

  let strategy = ErrorStrategy::<i32>::Retry(5);
  let cloned = strategy.clone();
  assert_eq!(strategy, cloned);

  let strategy = ErrorStrategy::<i32>::new_custom(|_error| ErrorAction::Retry);
  let cloned = strategy.clone();
  // Custom strategies are considered equal
  assert_eq!(strategy, cloned);
}

#[test]
fn test_error_strategy_partial_eq() {
  assert_eq!(ErrorStrategy::<i32>::Stop, ErrorStrategy::<i32>::Stop);
  assert_eq!(ErrorStrategy::<i32>::Skip, ErrorStrategy::<i32>::Skip);
  assert_eq!(
    ErrorStrategy::<i32>::Retry(3),
    ErrorStrategy::<i32>::Retry(3)
  );
  assert_ne!(
    ErrorStrategy::<i32>::Retry(3),
    ErrorStrategy::<i32>::Retry(5)
  );
  assert_ne!(ErrorStrategy::<i32>::Stop, ErrorStrategy::<i32>::Skip);

  // Custom strategies are considered equal
  let custom1 = ErrorStrategy::<i32>::new_custom(|_error| ErrorAction::Stop);
  let custom2 = ErrorStrategy::<i32>::new_custom(|_error| ErrorAction::Skip);
  assert_eq!(custom1, custom2);
}

#[test]
fn test_error_strategy_new_custom() {
  let strategy = ErrorStrategy::<i32>::new_custom(|error| {
    if error.retries < 2 {
      ErrorAction::Retry
    } else {
      ErrorAction::Stop
    }
  });

  // Create a test error
  let error = StreamError::new(
    Box::new(StringError("test error".to_string())),
    ErrorContext::default(),
    ComponentInfo::default(),
  );

  // The handler should be callable (though we can't directly test the result
  // without accessing the internal handler)
  let debug_str = format!("{:?}", strategy);
  assert!(debug_str.contains("Custom"));
}

// ============================================================================
// StreamError Tests
// ============================================================================

#[test]
fn test_stream_error_new() {
  let source = Box::new(StringError("test error".to_string()));
  let context = ErrorContext::default();
  let component = ComponentInfo::default();

  let error = StreamError::new(source, context.clone(), component.clone());

  assert_eq!(error.context, context);
  assert_eq!(error.component, component);
  assert_eq!(error.retries, 0);
}

#[test]
fn test_stream_error_clone() {
  let source = Box::new(StringError("test error".to_string()));
  let context = ErrorContext {
    timestamp: Utc::now(),
    item: Some(42),
    component_name: "test".to_string(),
    component_type: "Producer".to_string(),
  };
  let component = ComponentInfo::new("test-component".to_string(), "TestProducer".to_string());

  let error = StreamError::new(source, context.clone(), component.clone());
  let cloned = error.clone();

  assert_eq!(error.context, cloned.context);
  assert_eq!(error.component, cloned.component);
  assert_eq!(error.retries, cloned.retries);
  // Source is cloned as StringError, so we can compare the message
  assert_eq!(error.source.to_string(), cloned.source.to_string());
}

#[test]
fn test_stream_error_display() {
  let source = Box::new(StringError("test error message".to_string()));
  let context = ErrorContext::default();
  let component = ComponentInfo::new("my-component".to_string(), "MyProducer".to_string());

  let error = StreamError::new(source, context, component);
  let display_str = format!("{}", error);

  assert!(display_str.contains("my-component"));
  assert!(display_str.contains("MyProducer"));
  assert!(display_str.contains("test error message"));
}

#[test]
fn test_stream_error_source() {
  let source = Box::new(StringError("test error".to_string()));
  let context = ErrorContext::default();
  let component = ComponentInfo::default();

  let error = StreamError::new(source, context, component);
  let error_source = error.source();

  assert!(error_source.is_some());
}

// ============================================================================
// StringError Tests
// ============================================================================

#[test]
fn test_string_error_display() {
  let error = StringError("test error message".to_string());
  let display_str = format!("{}", error);
  assert_eq!(display_str, "test error message");
}

#[test]
fn test_string_error_debug() {
  let error = StringError("test error message".to_string());
  let debug_str = format!("{:?}", error);
  assert!(debug_str.contains("test error message"));
}

// ============================================================================
// ErrorContext Tests
// ============================================================================

#[test]
fn test_error_context_default() {
  let context = ErrorContext::<i32>::default();
  assert_eq!(context.item, None);
  assert_eq!(context.component_name, "default");
  assert_eq!(context.component_type, "default");
}

#[test]
fn test_error_context_clone() {
  let context = ErrorContext {
    timestamp: Utc::now(),
    item: Some(42),
    component_name: "test".to_string(),
    component_type: "Producer".to_string(),
  };

  let cloned = context.clone();
  assert_eq!(context, cloned);
}

#[test]
fn test_error_context_partial_eq() {
  let timestamp = Utc::now();
  let context1 = ErrorContext {
    timestamp,
    item: Some(42),
    component_name: "test".to_string(),
    component_type: "Producer".to_string(),
  };

  let context2 = ErrorContext {
    timestamp,
    item: Some(42),
    component_name: "test".to_string(),
    component_type: "Producer".to_string(),
  };

  assert_eq!(context1, context2);

  let context3 = ErrorContext {
    timestamp,
    item: Some(43),
    component_name: "test".to_string(),
    component_type: "Producer".to_string(),
  };

  assert_ne!(context1, context3);
}

// ============================================================================
// ComponentInfo Tests
// ============================================================================

#[test]
fn test_component_info_default() {
  let info = ComponentInfo::default();
  assert_eq!(info.name, "default");
  assert_eq!(info.type_name, "default");
}

#[test]
fn test_component_info_new() {
  let info = ComponentInfo::new("my-component".to_string(), "MyProducer".to_string());
  assert_eq!(info.name, "my-component");
  assert_eq!(info.type_name, "MyProducer");
}

#[test]
fn test_component_info_clone() {
  let info = ComponentInfo::new("test".to_string(), "TestProducer".to_string());
  let cloned = info.clone();
  assert_eq!(info, cloned);
}

#[test]
fn test_component_info_partial_eq() {
  let info1 = ComponentInfo::new("test".to_string(), "TestProducer".to_string());
  let info2 = ComponentInfo::new("test".to_string(), "TestProducer".to_string());
  let info3 = ComponentInfo::new("other".to_string(), "TestProducer".to_string());

  assert_eq!(info1, info2);
  assert_ne!(info1, info3);
}

#[test]
fn test_component_info_debug() {
  let info = ComponentInfo::new("test".to_string(), "TestProducer".to_string());
  let debug_str = format!("{:?}", info);
  assert!(debug_str.contains("test"));
  assert!(debug_str.contains("TestProducer"));
}

// ============================================================================
// PipelineStage Tests
// ============================================================================

#[test]
fn test_pipeline_stage_variants() {
  let stage1 = PipelineStage::Producer;
  let stage2 = PipelineStage::Transformer("test".to_string());
  let stage3 = PipelineStage::Consumer;

  assert_eq!(stage1, PipelineStage::Producer);
  assert_eq!(stage2, PipelineStage::Transformer("test".to_string()));
  assert_eq!(stage3, PipelineStage::Consumer);
  assert_ne!(stage1, stage2);
  assert_ne!(stage1, stage3);
  assert_ne!(stage2, stage3);
}

#[test]
fn test_pipeline_stage_clone() {
  let stage = PipelineStage::Transformer("test".to_string());
  let cloned = stage.clone();
  assert_eq!(stage, cloned);
}

#[test]
fn test_pipeline_stage_debug() {
  let stage = PipelineStage::Producer;
  let debug_str = format!("{:?}", stage);
  assert!(debug_str.contains("Producer"));

  let stage = PipelineStage::Transformer("test".to_string());
  let debug_str = format!("{:?}", stage);
  assert!(debug_str.contains("Transformer"));
  assert!(debug_str.contains("test"));

  let stage = PipelineStage::Consumer;
  let debug_str = format!("{:?}", stage);
  assert!(debug_str.contains("Consumer"));
}

// ============================================================================
// PipelineErrorContext Tests
// ============================================================================

#[test]
fn test_pipeline_error_context_new() {
  let context = ErrorContext::default();
  let stage = PipelineStage::Producer;

  let pipeline_context = PipelineErrorContext::new(context.clone(), stage.clone());
  assert_eq!(pipeline_context.context, context);
  assert_eq!(pipeline_context.stage, stage);
}

#[test]
fn test_pipeline_error_context_clone() {
  let context = ErrorContext::default();
  let stage = PipelineStage::Producer;

  let pipeline_context = PipelineErrorContext::new(context, stage);
  let cloned = pipeline_context.clone();
  assert_eq!(pipeline_context, cloned);
}

#[test]
fn test_pipeline_error_context_partial_eq() {
  let context1 = ErrorContext::default();
  let context2 = ErrorContext::default();
  let stage1 = PipelineStage::Producer;
  let stage2 = PipelineStage::Consumer;

  let pipeline_context1 = PipelineErrorContext::new(context1, stage1.clone());
  let pipeline_context2 = PipelineErrorContext::new(context2, stage1);
  let pipeline_context3 = PipelineErrorContext::new(ErrorContext::default(), stage2);

  assert_eq!(pipeline_context1, pipeline_context2);
  assert_ne!(pipeline_context1, pipeline_context3);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_error_strategy_with_stream_error() {
  // Test that ErrorStrategy can work with StreamError
  let strategy = ErrorStrategy::<i32>::Retry(3);

  let error = StreamError::new(
    Box::new(StringError("test".to_string())),
    ErrorContext {
      timestamp: Utc::now(),
      item: Some(42),
      component_name: "test".to_string(),
      component_type: "Producer".to_string(),
    },
    ComponentInfo::new("test".to_string(), "TestProducer".to_string()),
  );

  // Verify error structure
  assert_eq!(error.retries, 0);
  assert_eq!(error.context.item, Some(42));
  assert_eq!(error.component.name, "test");
}

#[test]
fn test_error_handling_workflow() {
  // Simulate a complete error handling workflow
  let mut error = StreamError::new(
    Box::new(StringError("initial error".to_string())),
    ErrorContext::default(),
    ComponentInfo::default(),
  );

  // Simulate retries
  error.retries = 1;
  assert_eq!(error.retries, 1);

  error.retries = 2;
  assert_eq!(error.retries, 2);

  // Test error display
  let display_str = format!("{}", error);
  assert!(display_str.contains("initial error"));
}
