//! # ONNX Runtime Integration
//!
//! This module provides ONNX Runtime backend implementation for machine learning inference.
//!
//! ONNX (Open Neural Network Exchange) is a portable model format that allows models
//! to be shared across different frameworks and platforms.
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::transformers::ml::{InferenceBackend, OnnxBackend};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut backend = OnnxBackend::new()?;
//! backend.load_from_path("model.onnx").await?;
//!
//! let input = vec![1.0, 2.0, 3.0];
//! let output = backend.infer(input).await?;
//! # Ok(())
//! # }
//! ```

use crate::transformers::ml::InferenceBackend;
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(feature = "ml")]
use ort::{
  environment::Environment,
  execution_providers::ExecutionProvider,
  session::Session,
  session::builder::{GraphOptimizationLevel, SessionBuilder},
  value::Value,
};

/// Error type for ONNX Runtime operations.
///
/// This wraps ONNX Runtime errors and provides context about what operation failed.
#[derive(Debug)]
pub struct OnnxError {
  message: String,
  source: Option<Box<dyn Error + Send + Sync>>,
}

impl OnnxError {
  /// Creates a new ONNX error with a message.
  pub fn new<S: Into<String>>(message: S) -> Self {
    Self {
      message: message.into(),
      source: None,
    }
  }

  /// Creates a new ONNX error with a message and source error.
  pub fn with_source<S: Into<String>, E: Error + Send + Sync + 'static>(
    message: S,
    source: E,
  ) -> Self {
    Self {
      message: message.into(),
      source: Some(Box::new(source)),
    }
  }
}

impl fmt::Display for OnnxError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "ONNX Runtime error: {}", self.message)?;
    if let Some(ref source) = self.source {
      write!(f, ": {}", source)?;
    }
    Ok(())
  }
}

impl Error for OnnxError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    self.source.as_ref().map(|e| e.as_ref() as &dyn Error)
  }
}

/// ONNX Runtime backend for machine learning inference.
///
/// This backend uses ONNX Runtime to load and run ONNX models for inference.
/// It supports both CPU and GPU execution (if available).
///
/// # Type Parameters
///
/// The input and output types are constrained to be vectors of f32, which is
/// the most common format for neural network inputs/outputs. For other types,
/// you may need to add conversion transformers before/after the inference transformer.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::transformers::ml::{InferenceBackend, OnnxBackend};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create backend with default settings (CPU)
/// let mut backend = OnnxBackend::new()?;
///
/// // Load model from file
/// backend.load_from_path("model.onnx").await?;
///
/// // Run inference
/// let input = vec![1.0, 2.0, 3.0, 4.0];
/// let output = backend.infer(input).await?;
///
/// // Use GPU if available
/// let mut gpu_backend = OnnxBackend::with_provider(ExecutionProvider::CUDA(Default::default()))?;
/// gpu_backend.load_from_path("model.onnx").await?;
/// # Ok(())
/// # }
/// ```
pub struct OnnxBackend {
  /// The ONNX Runtime session (wrapped in Arc<RwLock> for thread safety and hot-swapping)
  session: Arc<RwLock<Option<Session>>>,
  /// The execution provider (CPU, GPU, etc.)
  execution_provider: ExecutionProvider,
  /// Optimization level for graph optimization
  optimization_level: GraphOptimizationLevel,
}

impl OnnxBackend {
  /// Creates a new ONNX backend with default settings (CPU execution).
  ///
  /// # Returns
  ///
  /// Returns a new `OnnxBackend` instance configured for CPU execution.
  ///
  /// # Errors
  ///
  /// Returns an error if ONNX Runtime environment initialization fails.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// # use streamweave::transformers::ml::OnnxBackend;
  /// let backend = OnnxBackend::new()?;
  /// # Ok::<(), Box<dyn std::error::Error>>(())
  /// ```
  pub fn new() -> Result<Self, OnnxError> {
    Self::with_provider(ExecutionProvider::CPU(Default::default()))
  }

  /// Creates a new ONNX backend with a specific execution provider.
  ///
  /// # Arguments
  ///
  /// * `provider` - The execution provider to use (CPU, CUDA, TensorRT, etc.)
  ///
  /// # Returns
  ///
  /// Returns a new `OnnxBackend` instance configured with the specified execution provider.
  ///
  /// # Errors
  ///
  /// Returns an error if ONNX Runtime environment initialization fails.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// # use streamweave::transformers::ml::OnnxBackend;
  /// # use ort::ExecutionProvider;
  /// // Use CUDA if available
  /// let backend = OnnxBackend::with_provider(
  ///     ExecutionProvider::CUDA(Default::default())
  /// )?;
  /// # Ok::<(), Box<dyn std::error::Error>>(())
  /// ```
  pub fn with_provider(execution_provider: ExecutionProvider) -> Result<Self, OnnxError> {
    Ok(Self {
      session: Arc::new(RwLock::new(None)),
      execution_provider,
      optimization_level: GraphOptimizationLevel::All,
    })
  }

  /// Sets the graph optimization level.
  ///
  /// # Arguments
  ///
  /// * `level` - The optimization level (None, Basic, Extended, All)
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// # use streamweave::transformers::ml::OnnxBackend;
  /// # use ort::GraphOptimizationLevel;
  /// let mut backend = OnnxBackend::new()?;
  /// backend.set_optimization_level(GraphOptimizationLevel::All);
  /// # Ok::<(), Box<dyn std::error::Error>>(())
  /// ```
  pub fn set_optimization_level(&mut self, level: GraphOptimizationLevel) {
    self.optimization_level = level;
  }

  /// Gets the current optimization level.
  ///
  /// # Returns
  ///
  /// The current graph optimization level.
  pub fn optimization_level(&self) -> GraphOptimizationLevel {
    self.optimization_level
  }

  /// Hot-swaps the model by loading a new model and replacing the current session.
  ///
  /// This method atomically replaces the current model session with a new one,
  /// enabling zero-downtime model updates. The new model is loaded before the
  /// swap occurs, ensuring it's ready for inference.
  ///
  /// Since the session is wrapped in `Arc<RwLock<Option<Session>>>`, this method
  /// can be called even when the backend is shared across threads. It uses a
  /// write lock to atomically swap the session.
  ///
  /// # Arguments
  ///
  /// * `path` - The file system path to the new model file
  ///
  /// # Returns
  ///
  /// Returns `Ok(())` if the model was loaded and swapped successfully.
  ///
  /// # Errors
  ///
  /// Returns an error if:
  /// - The model file cannot be read
  /// - The model format is invalid
  /// - The new model fails to load
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// # use streamweave::transformers::ml::OnnxBackend;
  /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
  /// let mut backend = OnnxBackend::new()?;
  /// backend.load_from_path("model.onnx").await?;
  ///
  /// // Later, hot-swap to a new model (works even when backend is in Arc<RwLock>)
  /// backend.swap_model_from_path("new_model.onnx").await?;
  /// # Ok(())
  /// # }
  /// ```
  #[cfg(feature = "ml")]
  pub async fn swap_model_from_path(&self, path: &str) -> Result<(), OnnxError> {
    // Load the new model session first (outside the lock to minimize contention)
    let optimization_level = self.optimization_level;
    let execution_provider = self.execution_provider.clone();

    let new_session = SessionBuilder::new(&Environment::default().into_arc())
      .map_err(|e| OnnxError::with_source("Failed to create session builder", e))?
      .with_optimization_level(optimization_level)
      .map_err(|e| OnnxError::with_source("Failed to set optimization level", e))?
      .with_execution_providers([execution_provider])
      .map_err(|e| OnnxError::with_source("Failed to set execution provider", e))?
      .commit_from_file(path)
      .await
      .map_err(|e| {
        OnnxError::with_source(format!("Failed to load model from path: {}", path), e)
      })?;

    // Atomically swap the session
    let mut session_guard = self.session.write().await;
    *session_guard = Some(new_session);
    Ok(())
  }

  /// Hot-swaps the model by loading from bytes and replacing the current session.
  ///
  /// This method atomically replaces the current model session with a new one
  /// loaded from the provided bytes. Useful for loading models from memory or
  /// network sources.
  ///
  /// Since the session is wrapped in `Arc<RwLock<Option<Session>>>`, this method
  /// can be called even when the backend is shared across threads.
  ///
  /// # Arguments
  ///
  /// * `bytes` - The model file bytes
  ///
  /// # Returns
  ///
  /// Returns `Ok(())` if the model was loaded and swapped successfully.
  ///
  /// # Errors
  ///
  /// Returns an error if the model bytes are invalid or loading fails.
  #[cfg(feature = "ml")]
  pub async fn swap_model_from_bytes(&self, bytes: &[u8]) -> Result<(), OnnxError> {
    // Load the new model session first (outside the lock to minimize contention)
    let optimization_level = self.optimization_level;
    let execution_provider = self.execution_provider.clone();

    let new_session = SessionBuilder::new(&Environment::default().into_arc())
      .map_err(|e| OnnxError::with_source("Failed to create session builder", e))?
      .with_optimization_level(optimization_level)
      .map_err(|e| OnnxError::with_source("Failed to set optimization level", e))?
      .with_execution_providers([execution_provider])
      .map_err(|e| OnnxError::with_source("Failed to set execution provider", e))?
      .commit_from_memory(bytes)
      .map_err(|e| OnnxError::with_source("Failed to load model from bytes", e))?;

    // Atomically swap the session
    let mut session_guard = self.session.write().await;
    *session_guard = Some(new_session);
    Ok(())
  }
}

#[cfg(feature = "ml")]
impl InferenceBackend for OnnxBackend {
  type Input = Vec<f32>;
  type Output = Vec<f32>;
  type Error = OnnxError;

  async fn load_from_path(&mut self, path: &str) -> Result<(), Self::Error> {
    let session = SessionBuilder::new(&Environment::default().into_arc())
      .map_err(|e| OnnxError::with_source("Failed to create session builder", e))?
      .with_optimization_level(self.optimization_level)
      .map_err(|e| OnnxError::with_source("Failed to set optimization level", e))?
      .with_execution_providers([self.execution_provider.clone()])
      .map_err(|e| OnnxError::with_source("Failed to set execution provider", e))?
      .commit_from_file(path)
      .await
      .map_err(|e| {
        OnnxError::with_source(format!("Failed to load model from path: {}", path), e)
      })?;

    let mut session_guard = self.session.write().await;
    *session_guard = Some(session);
    Ok(())
  }

  async fn load_from_bytes(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
    let session = SessionBuilder::new(&Environment::default().into_arc())
      .map_err(|e| OnnxError::with_source("Failed to create session builder", e))?
      .with_optimization_level(self.optimization_level)
      .map_err(|e| OnnxError::with_source("Failed to set optimization level", e))?
      .with_execution_providers([self.execution_provider.clone()])
      .map_err(|e| OnnxError::with_source("Failed to set execution provider", e))?
      .commit_from_memory(bytes)
      .map_err(|e| OnnxError::with_source("Failed to load model from bytes", e))?;

    let mut session_guard = self.session.write().await;
    *session_guard = Some(session);
    Ok(())
  }

  async fn infer(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
    let session_guard = self.session.read().await;
    let session = session_guard
      .as_ref()
      .ok_or_else(|| OnnxError::new("Model not loaded"))?;

    // Get input name from model (assumes single input for simplicity)
    let input_name = session
      .inputs
      .iter()
      .next()
      .ok_or_else(|| OnnxError::new("Model has no inputs"))?
      .name
      .clone();

    // Create input tensor from vector
    // Note: This is a simplified implementation. In production, you'd need to
    // handle different input shapes and types properly
    let input_shape = vec![1, input.len() as i64];
    let input_value = Value::from_array(
      session.allocator(),
      ndarray::Array2::from_shape_vec((1, input.len()), input)
        .map_err(|e| OnnxError::with_source("Failed to create input array", e))?,
    )
    .map_err(|e| OnnxError::with_source("Failed to create input value", e))?;

    // Run inference
    let outputs = session
      .run(vec![(input_name, input_value)])
      .map_err(|e| OnnxError::with_source("Failed to run inference", e))?;

    // Extract output (assumes single output)
    let output_value = outputs
      .into_iter()
      .next()
      .ok_or_else(|| OnnxError::new("Model produced no outputs"))?
      .1;

    // Convert output to vector
    let output_array = output_value
      .try_extract::<ndarray::Array2<f32>>()
      .map_err(|e| OnnxError::with_source("Failed to extract output", e))?;

    // Flatten 2D array to 1D vector
    Ok(output_array.into_raw_vec())
  }

  async fn infer_batch(&self, inputs: Vec<Self::Input>) -> Result<Vec<Self::Output>, Self::Error> {
    if inputs.is_empty() {
      return Ok(vec![]);
    }

    let session_guard = self.session.read().await;
    let session = session_guard
      .as_ref()
      .ok_or_else(|| OnnxError::new("Model not loaded"))?;

    // Get input name and batch size
    let input_name = session
      .inputs
      .iter()
      .next()
      .ok_or_else(|| OnnxError::new("Model has no inputs"))?
      .name
      .clone();

    let batch_size = inputs.len();
    let feature_size = inputs[0].len();

    // Validate all inputs have same size
    for (i, input) in inputs.iter().enumerate() {
      if input.len() != feature_size {
        return Err(OnnxError::new(format!(
          "Input {} has size {}, expected {}",
          i,
          input.len(),
          feature_size
        )));
      }
    }

    // Create batched input tensor
    let mut batched_data = Vec::with_capacity(batch_size * feature_size);
    for input in &inputs {
      batched_data.extend(input);
    }

    let input_shape = vec![batch_size as i64, feature_size as i64];
    let input_value = Value::from_array(
      session.allocator(),
      ndarray::Array2::from_shape_vec((batch_size, feature_size), batched_data)
        .map_err(|e| OnnxError::with_source("Failed to create batched input array", e))?,
    )
    .map_err(|e| OnnxError::with_source("Failed to create batched input value", e))?;

    // Run batch inference
    let outputs = session
      .run(vec![(input_name, input_value)])
      .map_err(|e| OnnxError::with_source("Failed to run batch inference", e))?;

    // Extract output
    let output_value = outputs
      .into_iter()
      .next()
      .ok_or_else(|| OnnxError::new("Model produced no outputs"))?
      .1;

    // Convert output to 2D array
    let output_array = output_value
      .try_extract::<ndarray::Array2<f32>>()
      .map_err(|e| OnnxError::with_source("Failed to extract batch output", e))?;

    // Split batch output into individual outputs
    let mut results = Vec::with_capacity(batch_size);
    for row in output_array.rows() {
      results.push(row.to_vec());
    }

    Ok(results)
  }

  fn is_loaded(&self) -> bool {
    // Note: This is a synchronous check. For async, we'd need to use try_read
    // but that's okay for this check
    self
      .session
      .try_read()
      .map_or(false, |guard| guard.is_some())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_onnx_backend_new() {
    let backend = OnnxBackend::new();
    assert!(backend.is_ok());
  }

  #[tokio::test]
  async fn test_onnx_backend_is_loaded() {
    let backend = OnnxBackend::new().unwrap();
    assert!(!backend.is_loaded());
  }

  #[tokio::test]
  async fn test_onnx_backend_optimization_level() {
    let mut backend = OnnxBackend::new().unwrap();
    backend.set_optimization_level(GraphOptimizationLevel::All);
    assert_eq!(backend.optimization_level(), GraphOptimizationLevel::All);
  }

  // Note: Additional tests would require actual ONNX model files
  // which are not included in the repository
}
