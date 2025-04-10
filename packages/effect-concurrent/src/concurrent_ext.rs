use std::error::Error as StdError;
use std::time::Duration;

use effect_core::effect::Effect;
use effect_resource::Guard;
use futures::future::Either;
use tokio::time::sleep;

use crate::ConcurrentError;

/// Extension trait for concurrent operations on Effect
pub trait ConcurrentExt<T, E: StdError + Send + Sync + 'static>: Sized {
  /// Race two effects, returning the first one to complete
  fn race<U>(self, other: Effect<U, E>) -> Effect<Either<T, U>, E>
  where
    T: Send + Sync + 'static,
    U: Send + Sync + 'static;

  /// Zip two effects together, returning a tuple of their results
  fn zip<U>(self, other: Effect<U, E>) -> Effect<(T, U), E>
  where
    T: Send + Sync + 'static,
    U: Send + Sync + 'static;

  /// Apply a function to the results of two effects
  fn zip_with<U, V, F>(self, other: Effect<U, E>, f: F) -> Effect<V, E>
  where
    T: Send + Sync + 'static,
    U: Send + Sync + 'static,
    V: Send + Sync + 'static,
    F: FnOnce(T, U) -> V + Send + Sync + 'static;

  /// Timeout an effect after a specified duration
  fn timeout(self, duration: Duration) -> Effect<T, ConcurrentError>
  where
    T: Send + Sync + 'static,
    E: Send + Sync + 'static;

  /// Retry an effect a specified number of times
  fn retry<F>(self, attempts: usize, f: F) -> Effect<T, ConcurrentError>
  where
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
    F: Fn() -> Effect<T, E> + Send + Sync + 'static,
  {
    Effect::new(async move {
      let mut last_error = None;
      let mut attempts_remaining = attempts;

      while attempts_remaining > 0 {
        match f().run().await {
          Ok(value) => return Ok(value),
          Err(e) => {
            last_error = Some(ConcurrentError::ResourceError(e.to_string()));
            attempts_remaining -= 1;
          }
        }
      }
      Err(last_error.unwrap_or(ConcurrentError::RetryExhausted))
    })
  }

  /// Execute an effect with a resource that is acquired before and released after
  fn with_resource<R, F>(self, guard: Guard<R, E>, f: F) -> Effect<T, ConcurrentError>
  where
    T: Send + Sync + 'static,
    R: Send + Sync + 'static,
    F: FnOnce(&R) -> Effect<T, E> + Send + Sync + 'static;
}

impl<T, E: StdError + Send + Sync + 'static> ConcurrentExt<T, E> for Effect<T, E> {
  fn race<U>(self, other: Effect<U, E>) -> Effect<Either<T, U>, E>
  where
    T: Send + Sync + 'static,
    U: Send + Sync + 'static,
  {
    Effect::new(async move {
      let mut this = self;
      let mut other = other;
      let result = futures::future::select(Box::pin(this.run()), Box::pin(other.run())).await;
      match result {
        Either::Left((Ok(t), _)) => Ok(Either::Left(t)),
        Either::Right((Ok(u), _)) => Ok(Either::Right(u)),
        Either::Left((Err(e), _)) => Err(e),
        Either::Right((Err(e), _)) => Err(e),
      }
    })
  }

  fn zip<U>(self, other: Effect<U, E>) -> Effect<(T, U), E>
  where
    T: Send + Sync + 'static,
    U: Send + Sync + 'static,
  {
    Effect::new(async move {
      let mut this = self;
      let mut other = other;
      let (t, u) = futures::future::join(Box::pin(this.run()), Box::pin(other.run())).await;
      Ok((t?, u?))
    })
  }

  fn zip_with<U, V, F>(self, other: Effect<U, E>, f: F) -> Effect<V, E>
  where
    T: Send + Sync + 'static,
    U: Send + Sync + 'static,
    V: Send + Sync + 'static,
    F: FnOnce(T, U) -> V + Send + Sync + 'static,
  {
    Effect::new(async move {
      let mut this = self;
      let mut other = other;
      let (t, u) = futures::future::join(Box::pin(this.run()), Box::pin(other.run())).await;
      Ok(f(t?, u?))
    })
  }

  fn timeout(self, duration: Duration) -> Effect<T, ConcurrentError>
  where
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
  {
    Effect::new(async move {
      let mut this = self;
      tokio::select! {
        result = this.run() => match result {
          Ok(value) => Ok(value),
          Err(e) => Err(ConcurrentError::ResourceError(e.to_string())),
        },
        _ = sleep(duration) => Err(ConcurrentError::Timeout(duration)),
      }
    })
  }

  fn retry<F>(self, attempts: usize, f: F) -> Effect<T, ConcurrentError>
  where
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
    F: Fn() -> Effect<T, E> + Send + Sync + 'static,
  {
    Effect::new(async move {
      let mut last_error = None;
      let mut attempts_remaining = attempts;

      while attempts_remaining > 0 {
        match f().run().await {
          Ok(value) => return Ok(value),
          Err(e) => {
            last_error = Some(ConcurrentError::ResourceError(e.to_string()));
            attempts_remaining -= 1;
          }
        }
      }
      Err(last_error.unwrap_or(ConcurrentError::RetryExhausted))
    })
  }

  fn with_resource<R, F>(self, guard: Guard<R, E>, f: F) -> Effect<T, ConcurrentError>
  where
    Self: Sized,
    T: Send + Sync + 'static,
    R: Send + Sync + 'static,
    F: FnOnce(&R) -> Effect<T, E> + Send + Sync + 'static,
  {
    Effect::new(async move {
      let result = f(&guard).run().await;
      guard
        .release()
        .await
        .map_err(|e| ConcurrentError::ResourceError(e.to_string()))?;
      match result {
        Ok(value) => Ok(value),
        Err(e) => Err(ConcurrentError::ResourceError(e.to_string())),
      }
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::{Arc, Mutex};
  use std::time::Duration;
  use tokio_test::assert_ok;

  #[tokio::test]
  async fn test_race() {
    let fast = Effect::<_, std::io::Error>::pure(1);
    let slow = Effect::new(async {
      tokio::time::sleep(Duration::from_millis(100)).await;
      Ok(2)
    });

    let result = fast.race(slow).run().await;
    let either = result.unwrap();
    assert!(matches!(either, Either::Left(1)));
  }

  #[tokio::test]
  async fn test_zip() {
    let a = Effect::<_, std::io::Error>::pure(1);
    let b = Effect::pure(2);

    let result = a.zip(b).run().await;
    assert_ok!(result);
    assert_eq!(result.unwrap(), (1, 2));
  }

  #[tokio::test]
  async fn test_timeout() {
    let slow = Effect::<_, std::io::Error>::new(async {
      tokio::time::sleep(Duration::from_millis(100)).await;
      Ok(1)
    });

    let result = slow.timeout(Duration::from_millis(50)).run().await;
    assert!(matches!(result, Err(ConcurrentError::Timeout(_))));
  }

  #[tokio::test]
  async fn test_retry() {
    let attempts = Arc::new(Mutex::new(0));
    let attempts_effect = attempts.clone();
    let attempts_retry = attempts.clone();

    let effect = Effect::<_, std::io::Error>::new(async move {
      let mut attempts = attempts_effect.lock().unwrap();
      *attempts += 1;
      if *attempts < 3 {
        Err(std::io::Error::new(std::io::ErrorKind::Other, "retry"))
      } else {
        Ok(1)
      }
    });

    let result = effect
      .retry(3, move || {
        let attempts = attempts_retry.clone();
        Effect::new(async move {
          let mut attempts = attempts.lock().unwrap();
          *attempts += 1;
          if *attempts < 3 {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "retry"))
          } else {
            Ok(1)
          }
        })
      })
      .run()
      .await;
    assert_ok!(result);
    assert_eq!(result.unwrap(), 1);
    assert_eq!(*attempts.lock().unwrap(), 3);
  }

  #[tokio::test]
  async fn test_with_resource() {
    let resource = 42;
    let guard = Guard::new(resource, |_| Effect::pure(()));
    let effect = Effect::<_, std::io::Error>::pure(1);
    let result = effect
      .with_resource(guard, |r| Effect::pure(*r))
      .run()
      .await;
    assert_ok!(result);
    assert_eq!(result.unwrap(), 42);
  }
}
