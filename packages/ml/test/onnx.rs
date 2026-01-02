//! Tests for ONNX Runtime integration
//!
//! These tests verify the ONNX backend functionality without requiring actual ONNX model files.
//! Most tests focus on error handling, backend creation, and state management.

#[cfg(feature = "onnx")]
use ort::session::builder::GraphOptimizationLevel;
#[cfg(feature = "onnx")]
use std::error::Error;
#[cfg(feature = "onnx")]
use streamweave_ml::{InferenceBackend, OnnxBackend, OnnxError};

// Tests for OnnxError (require onnx feature since module is feature-gated)
#[cfg(feature = "onnx")]
#[test]
fn test_onnx_error_new() {
  let error = OnnxError::new("Test error message");
  assert_eq!(error.to_string(), "ONNX Runtime error: Test error message");
}

#[cfg(feature = "onnx")]
#[test]
fn test_onnx_error_with_source() {
  let source_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
  let error = OnnxError::with_source("Failed to load model", source_error);

  let error_string = error.to_string();
  assert!(error_string.contains("ONNX Runtime error: Failed to load model"));
  assert!(error_string.contains("File not found"));

  // Test that source is accessible
  assert!(error.source().is_some());
}

#[cfg(feature = "onnx")]
#[test]
fn test_onnx_error_display_formatting() {
  let error1 = OnnxError::new("Simple error");
  assert_eq!(format!("{}", error1), "ONNX Runtime error: Simple error");

  let source = std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid data");
  let error2 = OnnxError::with_source("Complex error", source);
  let display = format!("{}", error2);
  assert!(display.starts_with("ONNX Runtime error: Complex error"));
  assert!(display.contains("Invalid data"));
}

#[cfg(feature = "onnx")]
#[test]
fn test_onnx_error_source_chain() {
  let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Permission denied");
  let onnx_error = OnnxError::with_source("Model loading failed", io_error);

  // Verify the source error chain
  let source = onnx_error.source().unwrap();
  assert_eq!(source.to_string(), "Permission denied");
}

#[cfg(feature = "onnx")]
#[test]
fn test_onnx_error_without_source() {
  let error = OnnxError::new("No source error");
  assert!(error.source().is_none());
}

#[cfg(feature = "onnx")]
mod backend_tests {
  use super::GraphOptimizationLevel;
  use super::{Error, InferenceBackend, OnnxBackend, OnnxError};
  use proptest::prelude::*;

  #[tokio::test]
  async fn test_onnx_backend_new() {
    let backend_result = OnnxBackend::new();
    // Backend creation should succeed (even without a model loaded)
    assert!(backend_result.is_ok());
  }

  #[tokio::test]
  async fn test_onnx_backend_is_loaded_initially() {
    let backend = OnnxBackend::new().expect("Failed to create backend");
    // Backend should not be loaded initially
    assert!(!backend.is_loaded());
  }

  #[tokio::test]
  async fn test_onnx_backend_optimization_level_default() {
    let backend = OnnxBackend::new().expect("Failed to create backend");
    // Check that we can get the optimization level
    let level = backend.optimization_level();
    // Use std::mem::discriminant to compare enum variants
    use std::mem;
    assert!(mem::discriminant(level) == mem::discriminant(&GraphOptimizationLevel::Level1));
  }

  #[tokio::test]
  async fn test_onnx_backend_set_optimization_level() {
    use std::mem;
    let mut backend = OnnxBackend::new().expect("Failed to create backend");

    // Test setting different optimization levels
    backend.set_optimization_level(GraphOptimizationLevel::Level1);
    assert!(
      mem::discriminant(backend.optimization_level())
        == mem::discriminant(&GraphOptimizationLevel::Level1)
    );

    backend.set_optimization_level(GraphOptimizationLevel::Level2);
    assert!(
      mem::discriminant(backend.optimization_level())
        == mem::discriminant(&GraphOptimizationLevel::Level2)
    );

    backend.set_optimization_level(GraphOptimizationLevel::Level3);
    assert!(
      mem::discriminant(backend.optimization_level())
        == mem::discriminant(&GraphOptimizationLevel::Level3)
    );
  }

  #[tokio::test]
  async fn test_onnx_backend_load_from_path_nonexistent() {
    let mut backend = OnnxBackend::new().expect("Failed to create backend");

    // Attempting to load a non-existent model should fail
    let result = backend.load_from_path("nonexistent_model.onnx").await;
    assert!(result.is_err());

    // Backend should still not be loaded after failed load
    assert!(!backend.is_loaded());
  }

  #[tokio::test]
  async fn test_onnx_backend_load_from_bytes_invalid() {
    let mut backend = OnnxBackend::new().expect("Failed to create backend");

    // Attempting to load invalid bytes should fail
    let invalid_bytes = vec![0u8, 1u8, 2u8, 3u8];
    let result = backend.load_from_bytes(&invalid_bytes).await;
    assert!(result.is_err());

    // Backend should still not be loaded after failed load
    assert!(!backend.is_loaded());
  }

  #[tokio::test]
  async fn test_onnx_backend_infer_without_model() {
    let backend = OnnxBackend::new().expect("Failed to create backend");

    // Attempting inference without a loaded model should fail
    let input = vec![1.0, 2.0, 3.0];
    let result = backend.infer(input).await;
    assert!(result.is_err());

    // Verify error message indicates model not loaded
    if let Err(e) = result {
      let error_msg = e.to_string();
      assert!(error_msg.contains("Model not loaded") || error_msg.contains("model"));
    }
  }

  #[tokio::test]
  async fn test_onnx_backend_infer_batch_empty() {
    let backend = OnnxBackend::new().expect("Failed to create backend");

    // Empty batch should return empty result (even without model)
    // This tests the early return in infer_batch
    let result = backend.infer_batch(vec![]).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Vec::<Vec<f32>>::new());
  }

  #[tokio::test]
  async fn test_onnx_backend_infer_batch_without_model() {
    let backend = OnnxBackend::new().expect("Failed to create backend");

    // Attempting batch inference without a loaded model should fail
    let inputs = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
    let result = backend.infer_batch(inputs).await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_onnx_backend_swap_model_from_path_nonexistent() {
    let backend = OnnxBackend::new().expect("Failed to create backend");

    // Attempting to swap with a non-existent model should fail
    let result = backend.swap_model_from_path("nonexistent_model.onnx").await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_onnx_backend_swap_model_from_bytes_invalid() {
    let backend = OnnxBackend::new().expect("Failed to create backend");

    // Attempting to swap with invalid bytes should fail
    let invalid_bytes = vec![0u8, 1u8, 2u8, 3u8];
    let result = backend.swap_model_from_bytes(&invalid_bytes).await;
    assert!(result.is_err());
  }

  // Property-based tests using proptest
  proptest! {
    #[test]
    fn test_onnx_error_message_roundtrip(message in ".*") {
      let error = OnnxError::new(message.clone());
      let error_string = error.to_string();
      assert!(error_string.contains(&message));
      assert!(error_string.starts_with("ONNX Runtime error:"));
    }

    #[test]
    fn test_onnx_error_with_various_sources(
      error_message in ".*",
      io_kind in prop::sample::select(vec![
        std::io::ErrorKind::NotFound,
        std::io::ErrorKind::PermissionDenied,
        std::io::ErrorKind::InvalidData,
        std::io::ErrorKind::UnexpectedEof,
      ])
    ) {
      let source_error = std::io::Error::new(io_kind, error_message.clone());
      let onnx_error = OnnxError::with_source("Test error", source_error);

      let display = onnx_error.to_string();
      assert!(display.contains("Test error"));
      assert!(display.contains(&error_message));
      assert!(onnx_error.source().is_some());
    }

  }
}
