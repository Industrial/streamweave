//! Hot-swap tests
//!
//! Tests for model hot-swapping functionality.

use std::sync::Arc;
use streamweave_ml::*;
use tokio::sync::RwLock;

// Mock backend for testing
#[derive(Clone, Debug)]
struct MockBackend {
  loaded: bool,
  value: i32,
}

impl InferenceBackend for MockBackend {
  type Input = i32;
  type Output = i32;
  type Error = String;

  async fn load_from_path(&mut self, _path: &str) -> Result<(), Self::Error> {
    self.loaded = true;
    Ok(())
  }

  async fn load_from_bytes(&mut self, _bytes: &[u8]) -> Result<(), Self::Error> {
    self.loaded = true;
    Ok(())
  }

  async fn infer(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
    if !self.loaded {
      return Err("Model not loaded".to_string());
    }
    Ok(input + self.value)
  }

  async fn infer_batch(&self, inputs: Vec<Self::Input>) -> Result<Vec<Self::Output>, Self::Error> {
    if !self.loaded {
      return Err("Model not loaded".to_string());
    }
    Ok(inputs.into_iter().map(|x| x + self.value).collect())
  }

  fn is_loaded(&self) -> bool {
    self.loaded
  }

  fn input_shape(&self) -> Option<Vec<usize>> {
    Some(vec![1])
  }

  fn output_shape(&self) -> Option<Vec<usize>> {
    Some(vec![1])
  }
}

#[tokio::test]
async fn test_swap_model_success() {
  let mut backend1 = MockBackend {
    loaded: true,
    value: 10,
  };
  let backend_ref: Arc<RwLock<MockBackend>> = Arc::new(RwLock::new(backend1));

  let mut backend2 = MockBackend {
    loaded: true,
    value: 20,
  };

  // Swap should succeed
  let result = swap_model(&backend_ref, backend2).await;
  assert!(result.is_ok());

  // Verify the swap
  let guard = backend_ref.read().await;
  assert_eq!(guard.value, 20);
}

#[tokio::test]
async fn test_swap_model_not_loaded() {
  let mut backend1 = MockBackend {
    loaded: true,
    value: 10,
  };
  let backend_ref: Arc<RwLock<MockBackend>> = Arc::new(RwLock::new(backend1));

  let backend2 = MockBackend {
    loaded: false, // Not loaded
    value: 20,
  };

  // Swap should fail because new backend is not loaded
  let result = swap_model(&backend_ref, backend2).await;
  assert!(result.is_err());
  assert!(matches!(result.unwrap_err(), SwapError::ModelNotLoaded));
}

#[tokio::test]
async fn test_swap_model_concurrent_access() {
  let mut backend1 = MockBackend {
    loaded: true,
    value: 10,
  };
  let backend_ref: Arc<RwLock<MockBackend>> = Arc::new(RwLock::new(backend1));

  // Spawn a task that reads from the backend
  let backend_ref_clone = Arc::clone(&backend_ref);
  let read_task = tokio::spawn(async move {
    let guard = backend_ref_clone.read().await;
    guard.value
  });

  // Swap the backend
  let mut backend2 = MockBackend {
    loaded: true,
    value: 20,
  };
  swap_model(&backend_ref, backend2).await.unwrap();

  // Read task should complete (may read old or new value depending on timing)
  let _value = read_task.await.unwrap();
}

#[test]
fn test_swap_error_display() {
  let err = SwapError::ModelNotLoaded;
  assert!(err.to_string().contains("not loaded"));

  let err = SwapError::LockFailed("test".to_string());
  assert!(err.to_string().contains("Lock"));
}

#[test]
fn test_swap_error_error_trait() {
  let err = SwapError::ModelNotLoaded;
  let _: &dyn std::error::Error = &err;
}
