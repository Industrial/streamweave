//! Resource scoping implementation.
//!
//! This module provides the `ResourceScope` type for managing the scope of
//! resources in the effect system. It ensures that resources are properly
//! cleaned up when they go out of scope.

use std::error::Error as StdError;
use std::future::Future;
use std::sync::Arc;
use std::sync::Mutex;

use effect_core::effect::Effect;

/// A scope for managing resources.
pub struct ResourceScope<E: StdError + Send + Sync + 'static> {
  resources: Arc<Mutex<Vec<Box<dyn FnOnce() -> Effect<(), E> + Send + Sync>>>>,
}

impl<E: StdError + Send + Sync + 'static> ResourceScope<E> {
  /// Creates a new resource scope.
  pub fn new() -> Self {
    Self {
      resources: Arc::new(Mutex::new(Vec::new())),
    }
  }

  /// Adds a resource to the scope.
  pub fn add_resource<F>(&self, release: F)
  where
    F: FnOnce() -> Effect<(), E> + Send + Sync + 'static,
  {
    self.resources.lock().unwrap().push(Box::new(release));
  }

  /// Runs the scope, releasing all resources.
  pub fn run<F, T>(self, f: F) -> Effect<T, E>
  where
    F: FnOnce() -> Effect<T, E> + Send + Sync + 'static,
    T: Send + Sync + 'static,
  {
    Effect::new(async move {
      let result = f().run().await;
      let mut resources = self.resources.lock().unwrap();
      for release in resources.drain(..) {
        release().run().await?;
      }
      result
    })
  }
}

impl<E: StdError + Send + Sync + 'static> Default for ResourceScope<E> {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::{Error as IoError, ErrorKind};

  #[tokio::test]
  async fn test_resource_scope() {
    let mut released = false;
    let scope = ResourceScope::<IoError>::new();
    scope.add_resource(|| {
      released = true;
      Effect::pure(())
    });

    let effect = scope.run(|| Effect::pure(42));
    assert_eq!(effect.run().await, Ok(42));
    assert!(released);
  }

  #[tokio::test]
  async fn test_resource_scope_with_error() {
    let mut released = false;
    let scope = ResourceScope::<IoError>::new();
    scope.add_resource(|| {
      released = true;
      Effect::pure(())
    });

    let effect =
      scope.run(|| Effect::new(async move { Err(IoError::new(ErrorKind::Other, "test error")) }));
    assert!(effect.run().await.is_err());
    assert!(released);
  }

  #[tokio::test]
  async fn test_resource_scope_release_error() {
    let mut first_released = false;
    let mut second_released = false;
    let scope = ResourceScope::<IoError>::new();

    // First resource releases successfully
    scope.add_resource(|| {
      first_released = true;
      Effect::pure(())
    });

    // Second resource fails to release
    scope.add_resource(|| {
      second_released = true;
      Effect::new(async { Err(IoError::new(ErrorKind::Other, "release error")) })
    });

    let effect = scope.run(|| Effect::pure(42));
    let result = effect.run().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "release error");
    assert!(first_released);
    assert!(second_released);
  }

  #[tokio::test]
  async fn test_resource_scope_multiple_resources() {
    let mut releases = vec![false; 3];
    let scope = ResourceScope::<IoError>::new();

    for i in 0..3 {
      let releases = &mut releases;
      scope.add_resource(move || {
        releases[i] = true;
        Effect::pure(())
      });
    }

    let effect = scope.run(|| Effect::pure(42));
    assert_eq!(effect.run().await, Ok(42));
    assert!(releases.iter().all(|&released| released));
  }
}
