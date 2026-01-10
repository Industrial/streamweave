//! # Model Hot-Swapping
//!
//! This module provides utilities for hot-swapping ML models at runtime without
//! restarting the pipeline. This enables zero-downtime model updates.
//!
//! Hot-swapping works by replacing the backend instance atomically using the
//! `Arc<RwLock<B>>` pattern. New inference requests will use the new model,
//! while in-flight requests complete with the old model.
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave_ml_transformers::{OnnxBackend, InferenceTransformer};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut backend = OnnxBackend::new()?;
//! backend.load_from_path("model.onnx").await?;
//!
//! let transformer = InferenceTransformer::new(backend);
//! let backend_ref = transformer.backend();
//!
//! // Later, swap to a new model
//! let mut new_backend = OnnxBackend::new()?;
//! new_backend.load_from_path("new_model.onnx").await?;
//!
//! // Atomically swap the backend
//! let mut guard = backend_ref.write().await;
//! *guard = new_backend;
//! drop(guard);
//! # Ok(())
//! # }
//! ```

use super::InferenceBackend;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Swaps the model in a backend reference with a new backend instance.
///
/// This function atomically replaces the backend, ensuring no in-flight
/// inference requests are interrupted. New requests will use the new model.
///
/// The backend reference must be wrapped in `Arc<RwLock<B>>` (which is the
/// default for transformers). The new backend must have its model loaded
/// before calling this function.
///
/// # Arguments
///
/// * `backend_ref` - A reference to the backend wrapped in `Arc<RwLock<B>>`
/// * `new_backend` - The new backend instance with the updated model
///
/// # Returns
///
/// Returns `Ok(())` if the swap was successful, or an error if the swap failed.
///
/// # Errors
///
/// This function may return an error if:
/// - The new backend is not loaded
/// - The lock cannot be acquired
///
/// # Example
///
/// ```rust,no_run
/// # use streamweave_ml_transformers::{OnnxBackend, InferenceTransformer, swap_model};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut backend = OnnxBackend::new()?;
/// backend.load_from_path("model.onnx").await?;
///
/// let transformer = InferenceTransformer::new(backend);
/// let backend_ref = transformer.backend();
///
/// // Load new model
/// let mut new_backend = OnnxBackend::new()?;
/// new_backend.load_from_path("new_model.onnx").await?;
///
/// // Swap models atomically
/// swap_model(&backend_ref, new_backend).await?;
/// # Ok(())
/// # }
/// ```
pub async fn swap_model<B: InferenceBackend>(
  backend_ref: &Arc<RwLock<B>>,
  new_backend: B,
) -> Result<(), SwapError> {
  // Verify the new backend is loaded
  if !new_backend.is_loaded() {
    return Err(SwapError::ModelNotLoaded);
  }

  // Atomically swap the backend
  // The write lock ensures no inference is happening during the swap
  let mut guard = backend_ref.write().await;
  *guard = new_backend;
  drop(guard);

  Ok(())
}

/// Error type for model hot-swapping operations.
#[derive(Debug)]
pub enum SwapError {
  /// The new model is not loaded before swapping
  ModelNotLoaded,
  /// Failed to create a new backend instance
  BackendCreationFailed(String),
  /// Failed to load the model from the path
  ModelLoadFailed(String),
  /// Failed to acquire lock for swapping
  LockFailed(String),
}

impl std::fmt::Display for SwapError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SwapError::ModelNotLoaded => {
        write!(f, "Cannot swap: new model is not loaded")
      }
      SwapError::BackendCreationFailed(msg) => {
        write!(f, "Failed to create new backend: {}", msg)
      }
      SwapError::ModelLoadFailed(msg) => {
        write!(f, "Failed to load model: {}", msg)
      }
      SwapError::LockFailed(msg) => {
        write!(f, "Failed to acquire lock: {}", msg)
      }
    }
  }
}

impl std::error::Error for SwapError {}

// Note: The above implementation has a limitation - we can't easily create
// a new backend of generic type B without knowing its concrete type.
// Let me provide a better implementation that works with the actual backends.
