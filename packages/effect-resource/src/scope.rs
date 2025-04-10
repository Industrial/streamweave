//! Resource scope implementation.
//!
//! This module provides the `Scope` type for managing resource lifetimes
//! in the effect system. It ensures that resources are properly released
//! when the scope is exited.

use std::error::Error as StdError;
use std::sync::Mutex;

use effect_core::effect::Effect;

/// A scope for managing resource lifetimes.
pub struct Scope<E: StdError + Send + Sync + 'static> {
  resources: Mutex<Vec<Box<dyn FnOnce() -> Effect<(), E> + Send + Sync>>>,
}

impl<E: StdError + Send + Sync + 'static> Scope<E> {
  /// Creates a new empty scope.
  pub fn new() -> Self {
    Self {
      resources: Mutex::new(Vec::new()),
    }
  }

  /// Adds a resource to the scope.
  pub fn add_resource<F>(&self, release: F)
  where
    F: FnOnce() -> Effect<(), E> + Send + Sync + 'static,
  {
    self.resources.lock().unwrap().push(Box::new(release));
  }

  /// Runs an effect within the scope.
  pub async fn run<T, F>(&self, f: F) -> Result<T, E>
  where
    F: FnOnce() -> Effect<T, E>,
    T: Send + 'static,
  {
    let result = f().run().await;
    let resources = std::mem::take(&mut *self.resources.lock().unwrap());

    for release in resources {
      release().run().await?;
    }

    result
  }
}

impl<E: StdError + Send + Sync + 'static> Default for Scope<E> {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use std::io::{Error as IoError, ErrorKind};
  use std::sync::atomic::{AtomicBool, Ordering};
  use std::sync::Arc;

  use super::*;
  use effect_core::effect::Effect;

  #[tokio::test]
  async fn test_scope_success() {
    let released = Arc::new(AtomicBool::new(false));
    let released_clone = released.clone();

    let scope = Scope::new();
    scope.add_resource(Box::new(move || {
      let released = released_clone.clone();
      Effect::new(async move {
        released.store(true, Ordering::SeqCst);
        Ok(())
      })
    }));

    let result = scope.run(|| Effect::<i32>::new(async { Ok(42) })).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
    assert!(released.load(Ordering::SeqCst));
  }

  #[tokio::test]
  async fn test_scope_error() {
    let released = Arc::new(AtomicBool::new(false));
    let released_clone = released.clone();

    let scope = Scope::new();
    scope.add_resource(Box::new(move || {
      let released = released_clone.clone();
      Effect::new(async move {
        released.store(true, Ordering::SeqCst);
        Ok(())
      })
    }));

    let result = scope
      .run(|| Effect::<()>::new(async { Err(IoError::new(ErrorKind::Other, "test error")) }))
      .await;

    assert!(result.is_err());
    assert!(released.load(Ordering::SeqCst));
  }

  #[tokio::test]
  async fn test_scope_resource_error() {
    let scope = Scope::new();
    scope.add_resource(Box::new(|| {
      Effect::new(async { Err(IoError::new(ErrorKind::Other, "resource error")) })
    }));

    let result = scope.run(|| Effect::<i32>::new(async { Ok(42) })).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.to_string(), "resource error");
  }
}
