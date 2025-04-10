//! Resource guard implementation.
//!
//! This module provides the `ResourceGuard` type for managing resources with
//! automatic cleanup. It ensures that resources are properly released when
//! the guard goes out of scope.

use std::error::Error as StdError;
use std::future::Future;
use std::ops::{Deref, DerefMut};

use effect_core::effect::Effect;

/// A guard for a resource that ensures it is properly released.
pub struct ResourceGuard<T, E: StdError + Send + Sync + 'static> {
  resource: T,
  release: Box<dyn FnOnce(T) -> Effect<(), E> + Send + Sync>,
}

impl<T, E: StdError + Send + Sync + 'static> ResourceGuard<T, E> {
  /// Creates a new resource guard.
  pub fn new<F>(resource: T, release: F) -> Self
  where
    F: FnOnce(T) -> Effect<(), E> + Send + Sync + 'static,
  {
    Self {
      resource,
      release: Box::new(release),
    }
  }

  /// Consumes the guard and releases the resource.
  pub fn release(self) -> Effect<(), E> {
    (self.release)(self.resource)
  }
}

impl<T, E: StdError + Send + Sync + 'static> Deref for ResourceGuard<T, E> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    &self.resource
  }
}

impl<T, E: StdError + Send + Sync + 'static> DerefMut for ResourceGuard<T, E> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.resource
  }
}

impl<T, E: StdError + Send + Sync + 'static> Drop for ResourceGuard<T, E> {
  fn drop(&mut self) {
    let resource = std::mem::replace(&mut self.resource, unsafe { std::mem::zeroed() });
    let release = std::mem::replace(&mut self.release, Box::new(|_| Effect::pure(())));
    tokio::spawn(async move {
      let _ = release(resource).run().await;
    });
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::{Error as IoError, ErrorKind};

  #[tokio::test]
  async fn test_resource_guard() {
    let mut released = false;
    let guard = ResourceGuard::new(42, |_| {
      released = true;
      Effect::pure(())
    });

    assert_eq!(*guard, 42);
    drop(guard);
    assert!(released);
  }

  #[tokio::test]
  async fn test_resource_guard_release() {
    let mut released = false;
    let guard = ResourceGuard::new(42, |_| {
      released = true;
      Effect::pure(())
    });

    assert_eq!(*guard, 42);
    guard.release().run().await.unwrap();
    assert!(released);
  }

  #[tokio::test]
  async fn test_resource_guard_error() {
    let guard = ResourceGuard::new(42, |_| {
      Effect::new(async { Err(IoError::new(ErrorKind::Other, "release error")) })
    });

    let result = guard.release().run().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "release error");
  }
}
