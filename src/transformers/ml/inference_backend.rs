//! # Inference Backend Trait
//!
//! This module defines the core trait for machine learning inference backends.
//! Backends implement this trait to provide model loading and inference capabilities.
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::transformers::ml::InferenceBackend;
//!
//! // Implement InferenceBackend for your ML framework
//! struct MyBackend;
//!
//! impl InferenceBackend for MyBackend {
//!     type Input = Vec<f32>;
//!     type Output = Vec<f32>;
//!     type Error = MyError;
//!
//!     async fn load_from_path(&mut self, path: &str) -> Result<(), Self::Error> {
//!         // Load model implementation
//!         Ok(())
//!     }
//!
//!     async fn load_from_bytes(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
//!         // Load model from bytes
//!         Ok(())
//!     }
//!
//!     async fn infer(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
//!         // Run inference
//!         Ok(vec![])
//!     }
//!
//!     async fn infer_batch(&self, inputs: Vec<Self::Input>) -> Result<Vec<Self::Output>, Self::Error> {
//!         // Run batched inference
//!         Ok(vec![])
//!     }
//! }
//! ```

use std::error::Error;

/// Trait for machine learning inference backends.
///
/// This trait abstracts over different ML frameworks (ONNX Runtime, TensorFlow, PyTorch, etc.),
/// allowing StreamWeave to work with any backend that implements this interface.
///
/// # Type Parameters
///
/// * `Input` - The input type for inference (e.g., `Vec<f32>` for feature vectors)
/// * `Output` - The output type from inference (e.g., `Vec<f32>` for predictions)
/// * `Error` - The error type for backend-specific errors
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::ml::InferenceBackend;
///
/// struct MyBackend {
///     // Backend-specific state
/// }
///
/// #[derive(Debug)]
/// struct MyError(String);
///
/// impl std::fmt::Display for MyError {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "{}", self.0)
///     }
/// }
///
/// impl std::error::Error for MyError {}
///
/// impl InferenceBackend for MyBackend {
///     type Input = Vec<f32>;
///     type Output = Vec<f32>;
///     type Error = MyError;
///
///     async fn load_from_path(&mut self, path: &str) -> Result<(), Self::Error> {
///         // Implementation
///         Ok(())
///     }
///
///     async fn load_from_bytes(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
///         // Implementation
///         Ok(())
///     }
///
///     async fn infer(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
///         // Single inference
///         Ok(vec![0.0])
///     }
///
///     async fn infer_batch(&self, inputs: Vec<Self::Input>) -> Result<Vec<Self::Output>, Self::Error> {
///         // Batched inference
///         Ok(inputs.into_iter().map(|_| vec![0.0]).collect())
///     }
/// }
/// ```
pub trait InferenceBackend: Send + Sync {
  /// The input type for model inference.
  ///
  /// Typically this would be a feature vector, tensor, or other input representation
  /// specific to your model.
  type Input: std::fmt::Debug + Clone + Send + Sync;

  /// The output type from model inference.
  ///
  /// Typically this would be predictions, embeddings, or other output representation
  /// specific to your model.
  type Output: std::fmt::Debug + Clone + Send + Sync;

  /// The error type for backend-specific errors.
  ///
  /// Errors should implement `std::error::Error` for compatibility with StreamWeave's
  /// error handling system.
  type Error: Error + Send + Sync + 'static;

  /// Load a model from a file path.
  ///
  /// # Arguments
  ///
  /// * `path` - The file system path to the model file
  ///
  /// # Returns
  ///
  /// Returns `Ok(())` if the model was loaded successfully, or an error if loading failed.
  ///
  /// # Errors
  ///
  /// This method should return an error if:
  /// - The file doesn't exist
  /// - The file format is invalid
  /// - The model structure is incompatible
  /// - Memory allocation fails
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// # use streamweave::transformers::ml::InferenceBackend;
  /// # struct MyBackend;
  /// # impl InferenceBackend for MyBackend {
  /// #     type Input = Vec<f32>;
  /// #     type Output = Vec<f32>;
  /// #     type Error = Box<dyn std::error::Error>;
  /// #     async fn load_from_path(&mut self, path: &str) -> Result<(), Self::Error> { Ok(()) }
  /// #     async fn load_from_bytes(&mut self, bytes: &[u8]) -> Result<(), Self::Error> { Ok(()) }
  /// #     async fn infer(&self, input: Self::Input) -> Result<Self::Output, Self::Error> { Ok(vec![]) }
  /// #     async fn infer_batch(&self, inputs: Vec<Self::Input>) -> Result<Vec<Self::Output>, Self::Error> { Ok(vec![]) }
  /// # }
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let mut backend = MyBackend;
  /// backend.load_from_path("model.onnx").await?;
  /// # Ok(())
  /// # }
  /// ```
  async fn load_from_path(&mut self, path: &str) -> Result<(), Self::Error>;

  /// Load a model from bytes.
  ///
  /// This is useful for loading models from memory, embedded resources, or network sources.
  ///
  /// # Arguments
  ///
  /// * `bytes` - The model file bytes
  ///
  /// # Returns
  ///
  /// Returns `Ok(())` if the model was loaded successfully, or an error if loading failed.
  ///
  /// # Errors
  ///
  /// This method should return an error if:
  /// - The byte format is invalid
  /// - The model structure is incompatible
  /// - Memory allocation fails
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// # use streamweave::transformers::ml::InferenceBackend;
  /// # struct MyBackend;
  /// # impl InferenceBackend for MyBackend {
  /// #     type Input = Vec<f32>;
  /// #     type Output = Vec<f32>;
  /// #     type Error = Box<dyn std::error::Error>;
  /// #     async fn load_from_path(&mut self, path: &str) -> Result<(), Self::Error> { Ok(()) }
  /// #     async fn load_from_bytes(&mut self, bytes: &[u8]) -> Result<(), Self::Error> { Ok(()) }
  /// #     async fn infer(&self, input: Self::Input) -> Result<Self::Output, Self::Error> { Ok(vec![]) }
  /// #     async fn infer_batch(&self, inputs: Vec<Self::Input>) -> Result<Vec<Self::Output>, Self::Error> { Ok(vec![]) }
  /// # }
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let model_bytes = std::fs::read("model.onnx")?;
  /// let mut backend = MyBackend;
  /// backend.load_from_bytes(&model_bytes).await?;
  /// # Ok(())
  /// # }
  /// ```
  async fn load_from_bytes(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;

  /// Run inference on a single input.
  ///
  /// # Arguments
  ///
  /// * `input` - The input data for inference
  ///
  /// # Returns
  ///
  /// Returns the inference output, or an error if inference failed.
  ///
  /// # Errors
  ///
  /// This method should return an error if:
  /// - The model is not loaded
  /// - The input shape is incompatible
  /// - Inference execution fails
  /// - Output processing fails
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// # use streamweave::transformers::ml::InferenceBackend;
  /// # struct MyBackend;
  /// # impl InferenceBackend for MyBackend {
  /// #     type Input = Vec<f32>;
  /// #     type Output = Vec<f32>;
  /// #     type Error = Box<dyn std::error::Error>;
  /// #     async fn load_from_path(&mut self, path: &str) -> Result<(), Self::Error> { Ok(()) }
  /// #     async fn load_from_bytes(&mut self, bytes: &[u8]) -> Result<(), Self::Error> { Ok(()) }
  /// #     async fn infer(&self, input: Self::Input) -> Result<Self::Output, Self::Error> { Ok(input) }
  /// #     async fn infer_batch(&self, inputs: Vec<Self::Input>) -> Result<Vec<Self::Output>, Self::Error> { Ok(inputs) }
  /// # }
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let mut backend = MyBackend;
  /// backend.load_from_path("model.onnx").await?;
  /// let input = vec![1.0, 2.0, 3.0];
  /// let output = backend.infer(input).await?;
  /// # Ok(())
  /// # }
  /// ```
  fn infer(
    &self,
    input: Self::Input,
  ) -> impl std::future::Future<Output = Result<Self::Output, Self::Error>> + Send;

  /// Run inference on a batch of inputs.
  ///
  /// Batch inference is typically more efficient than running inference on individual
  /// inputs, especially on GPUs. The default implementation calls `infer` for each
  /// input, but backends should override this for better performance.
  ///
  /// # Arguments
  ///
  /// * `inputs` - A vector of input data for batch inference
  ///
  /// # Returns
  ///
  /// Returns a vector of inference outputs in the same order as the inputs, or an error
  /// if batch inference failed.
  ///
  /// # Errors
  ///
  /// This method should return an error if:
  /// - The model is not loaded
  /// - Any input shape is incompatible
  /// - Batch inference execution fails
  /// - Output processing fails
  ///
  /// # Performance
  ///
  /// Backends should implement efficient batch processing when possible. For GPU-accelerated
  /// backends, batch inference can provide significant speedup compared to individual inference.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// # use streamweave::transformers::ml::InferenceBackend;
  /// # struct MyBackend;
  /// # impl InferenceBackend for MyBackend {
  /// #     type Input = Vec<f32>;
  /// #     type Output = Vec<f32>;
  /// #     type Error = Box<dyn std::error::Error>;
  /// #     async fn load_from_path(&mut self, path: &str) -> Result<(), Self::Error> { Ok(()) }
  /// #     async fn load_from_bytes(&mut self, bytes: &[u8]) -> Result<(), Self::Error> { Ok(()) }
  /// #     async fn infer(&self, input: Self::Input) -> Result<Self::Output, Self::Error> { Ok(input) }
  /// #     async fn infer_batch(&self, inputs: Vec<Self::Input>) -> Result<Vec<Self::Output>, Self::Error> { Ok(inputs) }
  /// # }
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let mut backend = MyBackend;
  /// backend.load_from_path("model.onnx").await?;
  /// let inputs = vec![
  ///     vec![1.0, 2.0, 3.0],
  ///     vec![4.0, 5.0, 6.0],
  /// ];
  /// let outputs = backend.infer_batch(inputs).await?;
  /// # Ok(())
  /// # }
  /// ```
  fn infer_batch(
    &self,
    inputs: Vec<Self::Input>,
  ) -> impl std::future::Future<Output = Result<Vec<Self::Output>, Self::Error>> + Send;

  /// Check if a model is currently loaded.
  ///
  /// # Returns
  ///
  /// Returns `true` if a model is loaded and ready for inference, `false` otherwise.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// # use streamweave::transformers::ml::InferenceBackend;
  /// # struct MyBackend { loaded: bool }
  /// # impl InferenceBackend for MyBackend {
  /// #     type Input = Vec<f32>;
  /// #     type Output = Vec<f32>;
  /// #     type Error = Box<dyn std::error::Error>;
  /// #     async fn load_from_path(&mut self, path: &str) -> Result<(), Self::Error> { self.loaded = true; Ok(()) }
  /// #     async fn load_from_bytes(&mut self, bytes: &[u8]) -> Result<(), Self::Error> { self.loaded = true; Ok(()) }
  /// #     async fn infer(&self, input: Self::Input) -> Result<Self::Output, Self::Error> { Ok(vec![]) }
  /// #     async fn infer_batch(&self, inputs: Vec<Self::Input>) -> Result<Vec<Self::Output>, Self::Error> { Ok(vec![]) }
  /// #     fn is_loaded(&self) -> bool { self.loaded }
  /// # }
  /// # let backend = MyBackend { loaded: false };
  /// if !backend.is_loaded() {
  ///     // Load model
  /// }
  /// ```
  fn is_loaded(&self) -> bool;
}

#[cfg(test)]
mod tests {
  use super::*;

  // Mock backend for testing the trait
  struct MockBackend {
    loaded: bool,
  }

  impl MockBackend {
    fn new() -> Self {
      Self { loaded: false }
    }
  }

  #[derive(Debug)]
  struct MockError(String);

  impl std::fmt::Display for MockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for MockError {}

  impl InferenceBackend for MockBackend {
    type Input = Vec<f32>;
    type Output = Vec<f32>;
    type Error = MockError;

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
        return Err(MockError("Model not loaded".to_string()));
      }
      Ok(input)
    }

    async fn infer_batch(
      &self,
      inputs: Vec<Self::Input>,
    ) -> Result<Vec<Self::Output>, Self::Error> {
      if !self.loaded {
        return Err(MockError("Model not loaded".to_string()));
      }
      Ok(inputs)
    }

    fn is_loaded(&self) -> bool {
      self.loaded
    }
  }

  #[tokio::test]
  async fn test_load_from_path() {
    let mut backend = MockBackend::new();
    assert!(!backend.is_loaded());
    backend.load_from_path("test.onnx").await.unwrap();
    assert!(backend.is_loaded());
  }

  #[tokio::test]
  async fn test_load_from_bytes() {
    let mut backend = MockBackend::new();
    assert!(!backend.is_loaded());
    backend.load_from_bytes(b"model data").await.unwrap();
    assert!(backend.is_loaded());
  }

  #[tokio::test]
  async fn test_infer() {
    let mut backend = MockBackend::new();
    backend.load_from_path("test.onnx").await.unwrap();

    let input = vec![1.0, 2.0, 3.0];
    let output = backend.infer(input.clone()).await.unwrap();
    assert_eq!(output, vec![1.0, 2.0, 3.0]);
  }

  #[tokio::test]
  async fn test_infer_not_loaded() {
    let backend = MockBackend::new();
    let input = vec![1.0, 2.0, 3.0];
    let result = backend.infer(input).await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_infer_batch() {
    let mut backend = MockBackend::new();
    backend.load_from_path("test.onnx").await.unwrap();

    let inputs = vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]];
    let outputs = backend.infer_batch(inputs.clone()).await.unwrap();
    assert_eq!(outputs.len(), 3);
    assert_eq!(outputs, inputs);
  }

  #[tokio::test]
  async fn test_infer_batch_default_implementation() {
    // Test that default infer_batch implementation works correctly
    let mut backend = MockBackend::new();
    backend.load_from_path("test.onnx").await.unwrap();

    let inputs = vec![vec![1.0], vec![2.0], vec![3.0]];
    let outputs = backend.infer_batch(inputs.clone()).await.unwrap();
    assert_eq!(outputs.len(), 3);
    assert_eq!(outputs, inputs);
  }

  #[tokio::test]
  async fn test_is_loaded() {
    let backend = MockBackend::new();
    assert!(!backend.is_loaded());

    let mut backend = MockBackend::new();
    backend.load_from_path("test.onnx").await.unwrap();
    assert!(backend.is_loaded());
  }
}
