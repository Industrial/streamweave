//! Resource guard implementation.
//!
//! This module provides the `Guard` type for managing resources with
//! automatic cleanup. It ensures that resources are properly released when
//! the guard goes out of scope.

use std::error::Error as StdError;
use std::ops::{Deref, DerefMut};

use effect_core::effect::Effect;

/// A guard for a resource that ensures it is properly released.
pub struct Guard<T: Send + 'static, E: StdError + Send + Sync + 'static> {
  resource: Option<T>,
  release: Option<Box<dyn FnOnce(T) -> Effect<(), E> + Send + Sync>>,
}

impl<T: Send + 'static, E: StdError + Send + Sync + 'static> Guard<T, E> {
  /// Creates a new resource guard.
  pub fn new<F>(resource: T, release: F) -> Self
  where
    F: FnOnce(T) -> Effect<(), E> + Send + Sync + 'static,
  {
    Self {
      resource: Some(resource),
      release: Some(Box::new(release)),
    }
  }

  /// Consumes the guard and releases the resource.
  pub fn release(mut self) -> Effect<(), E> {
    let resource = self.resource.take().unwrap();
    let release = self.release.take().unwrap();
    release(resource)
  }
}

impl<T: Send + 'static, E: StdError + Send + Sync + 'static> Deref for Guard<T, E> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    self.resource.as_ref().unwrap()
  }
}

impl<T: Send + 'static, E: StdError + Send + Sync + 'static> DerefMut for Guard<T, E> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.resource.as_mut().unwrap()
  }
}

impl<T: Send + 'static, E: StdError + Send + Sync + 'static> Drop for Guard<T, E> {
  fn drop(&mut self) {
    if let (Some(resource), Some(release)) = (self.resource.take(), self.release.take()) {
      // Try to get the current runtime handle
      match tokio::runtime::Handle::try_current() {
        Ok(rt) => {
          // We're in an async context, spawn a task
          let _ = rt.spawn(async move {
            let _ = release(resource).run().await;
          });
        }
        Err(_) => {
          // We're in a sync context, create a new runtime
          let rt = tokio::runtime::Runtime::new().unwrap();
          rt.block_on(async {
            let _ = release(resource).run().await;
          });
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::{Error as IoError, ErrorKind};
  use std::sync::atomic::{AtomicBool, Ordering};
  use std::sync::Arc;

  #[tokio::test]
  async fn test_guard() {
    let released = Arc::new(AtomicBool::new(false));
    let released_clone = Arc::clone(&released);
    {
      let guard = Guard::<_, IoError>::new(42, move |_| {
        released_clone.store(true, Ordering::SeqCst);
        Effect::pure(())
      });
      assert_eq!(*guard, 42);
    }
    // Give the spawned task a chance to complete
    tokio::task::yield_now().await;
    assert!(released.load(Ordering::SeqCst));
  }

  #[tokio::test]
  async fn test_guard_release() {
    let released = Arc::new(AtomicBool::new(false));
    let released_clone = Arc::clone(&released);
    let guard = Guard::<_, IoError>::new(42, move |_| {
      released_clone.store(true, Ordering::SeqCst);
      Effect::pure(())
    });

    assert_eq!(*guard, 42);
    guard.release().run().await.unwrap();
    assert!(released.load(Ordering::SeqCst));
  }

  #[tokio::test]
  async fn test_guard_error() {
    let guard = Guard::<_, IoError>::new(42, |_| {
      Effect::new(async { Err(IoError::new(ErrorKind::Other, "release error")) })
    });

    let result = guard.release().run().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "release error");
  }
}
