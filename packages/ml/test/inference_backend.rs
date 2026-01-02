//! Inference backend trait tests
//!
//! Tests for the InferenceBackend trait using mock implementations.

use streamweave_ml::*;

// Mock backend implementation for testing
#[derive(Clone, Debug)]
struct TestBackend {
  loaded: bool,
  multiplier: f32,
}

#[derive(Debug)]
struct TestError(String);

impl std::fmt::Display for TestError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl std::error::Error for TestError {}

impl InferenceBackend for TestBackend {
  type Input = Vec<f32>;
  type Output = Vec<f32>;
  type Error = TestError;

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
      return Err(TestError("Model not loaded".to_string()));
    }
    Ok(input.into_iter().map(|x| x * self.multiplier).collect())
  }

  async fn infer_batch(&self, inputs: Vec<Self::Input>) -> Result<Vec<Self::Output>, Self::Error> {
    if !self.loaded {
      return Err(TestError("Model not loaded".to_string()));
    }
    Ok(
      inputs
        .into_iter()
        .map(|input| input.into_iter().map(|x| x * self.multiplier).collect())
        .collect(),
    )
  }

  fn is_loaded(&self) -> bool {
    self.loaded
  }
}

#[tokio::test]
async fn test_backend_load_from_path() {
  let mut backend = TestBackend {
    loaded: false,
    multiplier: 2.0,
  };

  assert!(!backend.is_loaded());
  backend.load_from_path("test.model").await.unwrap();
  assert!(backend.is_loaded());
}

#[tokio::test]
async fn test_backend_load_from_bytes() {
  let mut backend = TestBackend {
    loaded: false,
    multiplier: 2.0,
  };

  assert!(!backend.is_loaded());
  let bytes = b"model data";
  backend.load_from_bytes(bytes).await.unwrap();
  assert!(backend.is_loaded());
}

#[tokio::test]
async fn test_backend_infer() {
  let mut backend = TestBackend {
    loaded: false,
    multiplier: 2.0,
  };

  // Should fail when not loaded
  let result = backend.infer(vec![1.0, 2.0, 3.0]).await;
  assert!(result.is_err());

  // Load and try again
  backend.load_from_path("test.model").await.unwrap();
  let result = backend.infer(vec![1.0, 2.0, 3.0]).await.unwrap();
  assert_eq!(result, vec![2.0, 4.0, 6.0]);
}

#[tokio::test]
async fn test_backend_infer_batch() {
  let mut backend = TestBackend {
    loaded: false,
    multiplier: 2.0,
  };

  // Should fail when not loaded
  let result = backend
    .infer_batch(vec![vec![1.0, 2.0], vec![3.0, 4.0]])
    .await;
  assert!(result.is_err());

  // Load and try again
  backend.load_from_path("test.model").await.unwrap();
  let result = backend
    .infer_batch(vec![vec![1.0, 2.0], vec![3.0, 4.0]])
    .await
    .unwrap();
  assert_eq!(result, vec![vec![2.0, 4.0], vec![6.0, 8.0]]);
}

#[tokio::test]
async fn test_backend_empty_batch() {
  let mut backend = TestBackend {
    loaded: false,
    multiplier: 2.0,
  };
  backend.load_from_path("test.model").await.unwrap();

  let result: Vec<Vec<f32>> = backend.infer_batch(vec![]).await.unwrap();
  assert_eq!(result, Vec::<Vec<f32>>::new());
}

#[tokio::test]
async fn test_backend_empty_input() {
  let mut backend = TestBackend {
    loaded: false,
    multiplier: 2.0,
  };
  backend.load_from_path("test.model").await.unwrap();

  let result: Vec<f32> = backend.infer(vec![]).await.unwrap();
  assert_eq!(result, vec![]);
}
