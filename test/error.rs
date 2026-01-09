//! Tests for error module

use std::io;
use streamweave::error::*;

#[test]
fn test_error_action_variants() {
  assert_eq!(ErrorAction::Stop, ErrorAction::Stop);
  assert_eq!(ErrorAction::Skip, ErrorAction::Skip);
  assert_eq!(ErrorAction::Retry, ErrorAction::Retry);
  assert_ne!(ErrorAction::Stop, ErrorAction::Skip);
}

#[test]
fn test_error_strategy_stop() {
  let strategy = ErrorStrategy::<i32>::Stop;
  let error = StreamError::new(
    Box::new(io::Error::new(io::ErrorKind::Other, "test")),
    ErrorContext::default(),
    ComponentInfo {
      name: "test".to_string(),
      type_name: "Test".to_string(),
    },
  );

  // Test that Stop strategy returns Stop action
  match strategy {
    ErrorStrategy::Stop => assert!(true),
    _ => assert!(false),
  }
}

#[test]
fn test_error_strategy_skip() {
  let strategy = ErrorStrategy::<i32>::Skip;
  match strategy {
    ErrorStrategy::Skip => assert!(true),
    _ => assert!(false),
  }
}

#[test]
fn test_error_strategy_retry() {
  let strategy = ErrorStrategy::<i32>::Retry(3);
  match strategy {
    ErrorStrategy::Retry(n) => assert_eq!(n, 3),
    _ => assert!(false),
  }
}

#[test]
fn test_error_strategy_custom() {
  let strategy = ErrorStrategy::<i32>::new_custom(|_| ErrorAction::Skip);
  match strategy {
    ErrorStrategy::Custom(_) => assert!(true),
    _ => assert!(false),
  }
}

#[test]
fn test_error_strategy_clone() {
  let strategy1 = ErrorStrategy::<i32>::Retry(5);
  let strategy2 = strategy1.clone();

  match (strategy1, strategy2) {
    (ErrorStrategy::Retry(n1), ErrorStrategy::Retry(n2)) => assert_eq!(n1, n2),
    _ => assert!(false),
  }
}

#[test]
fn test_error_strategy_partial_eq() {
  assert_eq!(ErrorStrategy::<i32>::Stop, ErrorStrategy::Stop);
  assert_eq!(ErrorStrategy::<i32>::Skip, ErrorStrategy::Skip);
  assert_eq!(ErrorStrategy::<i32>::Retry(3), ErrorStrategy::Retry(3));
  assert_ne!(ErrorStrategy::<i32>::Retry(3), ErrorStrategy::Retry(4));
}

#[test]
fn test_stream_error_new() {
  let error = StreamError::new(
    Box::new(io::Error::new(io::ErrorKind::NotFound, "File not found")),
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(42),
      component_name: "TestProducer".to_string(),
      component_type: "Producer".to_string(),
    },
    ComponentInfo {
      name: "test-producer".to_string(),
      type_name: "TestProducer".to_string(),
    },
  );

  assert_eq!(error.retries, 0);
  assert_eq!(error.context.item, Some(42));
  assert_eq!(error.component.name, "test-producer");
}

#[test]
fn test_stream_error_clone() {
  let error1 = StreamError::new(
    Box::new(io::Error::new(io::ErrorKind::Other, "test")),
    ErrorContext::default(),
    ComponentInfo {
      name: "test".to_string(),
      type_name: "Test".to_string(),
    },
  );
  let error2 = error1.clone();

  assert_eq!(error1.retries, error2.retries);
  assert_eq!(error1.component.name, error2.component.name);
}

#[test]
fn test_stream_error_display() {
  let error = StreamError::new(
    Box::new(io::Error::new(io::ErrorKind::Other, "test error")),
    ErrorContext::default(),
    ComponentInfo {
      name: "test".to_string(),
      type_name: "Test".to_string(),
    },
  );

  let display = format!("{}", error);
  assert!(display.contains("test"));
  assert!(display.contains("Test"));
}

#[test]
fn test_string_error() {
  let error = StringError("test error".to_string());
  assert_eq!(format!("{}", error), "test error");
}

#[test]
fn test_error_context_default() {
  let ctx = ErrorContext::<i32>::default();
  assert_eq!(ctx.item, None);
  assert_eq!(ctx.component_name, "default");
}

#[test]
fn test_error_context_clone() {
  let ctx1 = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: Some(42),
    component_name: "test".to_string(),
    component_type: "Test".to_string(),
  };
  let ctx2 = ctx1.clone();

  assert_eq!(ctx1.item, ctx2.item);
  assert_eq!(ctx1.component_name, ctx2.component_name);
}

#[test]
fn test_component_info() {
  let info = ComponentInfo {
    name: "test-component".to_string(),
    type_name: "TestComponent".to_string(),
  };

  assert_eq!(info.name, "test-component");
  assert_eq!(info.type_name, "TestComponent");
}

#[test]
fn test_component_info_clone() {
  let info1 = ComponentInfo {
    name: "test".to_string(),
    type_name: "Test".to_string(),
  };
  let info2 = info1.clone();

  assert_eq!(info1.name, info2.name);
  assert_eq!(info1.type_name, info2.type_name);
}

#[test]
fn test_pipeline_stage_variants() {
  assert_eq!(PipelineStage::Producer, PipelineStage::Producer);
  assert_eq!(
    PipelineStage::Transformer("test".to_string()),
    PipelineStage::Transformer("test".to_string())
  );
  assert_eq!(PipelineStage::Consumer, PipelineStage::Consumer);
}

#[test]
fn test_pipeline_error() {
  let error = PipelineError::new(
    io::Error::new(io::ErrorKind::Other, "test"),
    ErrorContext::default(),
    ComponentInfo {
      name: "test".to_string(),
      type_name: "Test".to_string(),
    },
  );

  assert_eq!(error.component().name, "test");
  assert_eq!(error.component().type_name, "Test");
}
