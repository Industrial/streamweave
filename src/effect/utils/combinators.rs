//! Effect combinators implementation.
//!
//! This module provides combinators for composing and transforming effects.

use std::error::Error as StdError;
use std::future::Future;

use crate::effect::core::effect::Effect;

/// Applies a function to the success value of an effect.
pub fn map<T, U, E, F>(effect: Effect<T, E>, f: F) -> Effect<U, E>
where
  T: Send + Sync + 'static,
  U: Send + Sync + 'static,
  E: StdError + Send + Sync + 'static,
  F: FnOnce(T) -> U + Send + Sync + 'static,
{
  effect.map(f)
}

/// Applies a function that returns an effect to the success value of an effect.
pub fn flat_map<T, U, E, F>(effect: Effect<T, E>, f: F) -> Effect<U, E>
where
  T: Send + Sync + 'static,
  U: Send + Sync + 'static,
  E: StdError + Send + Sync + 'static,
  F: FnOnce(T) -> Effect<U, E> + Send + Sync + 'static,
{
  effect.flat_map(f)
}

/// Zips two effects together, producing a tuple of their results.
pub fn zip<T1, T2, E>(effect1: Effect<T1, E>, effect2: Effect<T2, E>) -> Effect<(T1, T2), E>
where
  T1: Send + Sync + 'static,
  T2: Send + Sync + 'static,
  E: StdError + Send + Sync + 'static,
{
  Effect::new(async move {
    let result1 = effect1.run().await?;
    let result2 = effect2.run().await?;
    Ok((result1, result2))
  })
}

/// Retries an effect up to a maximum number of times.
pub fn retry<T, E, F>(f: F, max_retries: usize) -> Effect<T, E>
where
  T: Send + Sync + 'static,
  E: StdError + Send + Sync + 'static,
  F: Fn() -> Effect<T, E> + Send + Sync + 'static,
{
  Effect::new(async move {
    let mut retries = 0;
    loop {
      match f().run().await {
        Ok(result) => return Ok(result),
        Err(error) if retries < max_retries => {
          retries += 1;
          continue;
        }
        Err(error) => return Err(error),
      }
    }
  })
}

/// Delays an effect by a specified duration.
pub fn delay<T, E>(effect: Effect<T, E>, duration: std::time::Duration) -> Effect<T, E>
where
  T: Send + Sync + 'static,
  E: StdError + Send + Sync + 'static,
{
  Effect::new(async move {
    tokio::time::sleep(duration).await;
    effect.run().await
  })
}

/// Runs multiple effects in parallel and collects their results.
pub async fn join_all<T, E, I>(effects: I) -> Result<Vec<T>, E>
where
  I: IntoIterator<Item = Effect<T, E>>,
  T: Send + Sync + 'static,
  E: StdError + Send + Sync + 'static,
{
  let futures: Vec<_> = effects.into_iter().map(|e| e.run()).collect();
  futures::future::try_join_all(futures).await
}

/// Runs multiple effects in parallel and returns the first successful result.
pub async fn race_ok<T, E, I>(effects: I) -> Result<T, E>
where
  I: IntoIterator<Item = Effect<T, E>>,
  T: Send + Sync + 'static,
  E: StdError + Send + Sync + 'static,
{
  let futures: Vec<_> = effects.into_iter().map(|e| e.run()).collect();
  futures::future::select_ok(futures).await.map(|(t, _)| t)
}

/// Runs multiple effects in parallel and returns the first result (success or failure).
pub async fn race<T, E, I>(effects: I) -> Result<T, E>
where
  I: IntoIterator<Item = Effect<T, E>>,
  T: Send + Sync + 'static,
  E: StdError + Send + Sync + 'static,
{
  let futures: Vec<_> = effects.into_iter().map(|e| e.run()).collect();
  futures::future::select_all(futures).await.0
}

/// Runs multiple effects sequentially and collects their results.
pub async fn sequence<T, E, I>(effects: I) -> Result<Vec<T>, E>
where
  I: IntoIterator<Item = Effect<T, E>>,
  T: Send + Sync + 'static,
  E: StdError + Send + Sync + 'static,
{
  let mut results = Vec::new();
  for effect in effects {
    results.push(effect.run().await?);
  }
  Ok(results)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::{Error as IoError, ErrorKind};
  use std::sync::Arc;
  use std::sync::atomic::{AtomicUsize, Ordering};
  use std::time::{Duration, Instant};
  use tokio::time::sleep;

  // Map tests
  #[tokio::test]
  async fn test_map_success() {
    let effect = Effect::new(async { Ok::<_, IoError>(21) });
    let mapped = map(effect, |x| x * 2);
    assert_eq!(mapped.run().await, Ok(42));
  }

  #[tokio::test]
  async fn test_map_error() {
    let effect = Effect::new(async { Err::<i32, _>(IoError::new(ErrorKind::Other, "test error")) });
    let mapped = map(effect, |x| x * 2);
    assert!(mapped.run().await.is_err());
  }

  #[tokio::test]
  async fn test_map_different_types() {
    let effect = Effect::new(async { Ok::<_, IoError>(42) });
    let mapped = map(effect, |x| x.to_string());
    assert_eq!(mapped.run().await, Ok("42".to_string()));
  }

  // Flat map tests
  #[tokio::test]
  async fn test_flat_map_success() {
    let effect = Effect::new(async { Ok::<_, IoError>(21) });
    let mapped = flat_map(effect, |x| Effect::new(async move { Ok(x * 2) }));
    assert_eq!(mapped.run().await, Ok(42));
  }

  #[tokio::test]
  async fn test_flat_map_error() {
    let effect = Effect::new(async { Err::<i32, _>(IoError::new(ErrorKind::Other, "test error")) });
    let mapped = flat_map(effect, |x| Effect::new(async move { Ok(x * 2) }));
    assert!(mapped.run().await.is_err());
  }

  #[tokio::test]
  async fn test_flat_map_nested_error() {
    let effect = Effect::new(async { Ok::<_, IoError>(42) });
    let mapped = flat_map(effect, |_| {
      Effect::new(async { Err::<i32, _>(IoError::new(ErrorKind::Other, "nested error")) })
    });
    assert!(mapped.run().await.is_err());
  }

  // Zip tests
  #[tokio::test]
  async fn test_zip_success() {
    let effect1 = Effect::new(async { Ok::<_, IoError>(1) });
    let effect2 = Effect::new(async { Ok(2) });
    let zipped = zip(effect1, effect2);
    assert_eq!(zipped.run().await, Ok((1, 2)));
  }

  #[tokio::test]
  async fn test_zip_first_error() {
    let effect1 =
      Effect::new(async { Err::<i32, _>(IoError::new(ErrorKind::Other, "first error")) });
    let effect2 = Effect::new(async { Ok(2) });
    let zipped = zip(effect1, effect2);
    assert!(zipped.run().await.is_err());
  }

  #[tokio::test]
  async fn test_zip_second_error() {
    let effect1 = Effect::new(async { Ok::<_, IoError>(1) });
    let effect2 = Effect::new(async { Err(IoError::new(ErrorKind::Other, "second error")) });
    let zipped = zip(effect1, effect2);
    assert!(zipped.run().await.is_err());
  }

  #[tokio::test]
  async fn test_zip_different_types() {
    let effect1 = Effect::new(async { Ok::<_, IoError>("hello".to_string()) });
    let effect2 = Effect::new(async { Ok(42) });
    let zipped = zip(effect1, effect2);
    assert_eq!(zipped.run().await, Ok(("hello".to_string(), 42)));
  }

  // Retry tests
  #[tokio::test]
  async fn test_retry_success_first_try() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    let effect = retry(
      move || {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        Effect::new(async { Ok::<_, IoError>(42) })
      },
      3,
    );

    assert_eq!(effect.run().await, Ok(42));
    assert_eq!(counter.load(Ordering::SeqCst), 1);
  }

  #[tokio::test]
  async fn test_retry_success_after_retries() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    let effect = retry(
      move || {
        let count = counter_clone.fetch_add(1, Ordering::SeqCst);
        Effect::new(async move {
          if count < 2 {
            Err(IoError::new(ErrorKind::Other, "retry"))
          } else {
            Ok(42)
          }
        })
      },
      3,
    );

    assert_eq!(effect.run().await, Ok(42));
    assert_eq!(counter.load(Ordering::SeqCst), 3);
  }

  #[tokio::test]
  async fn test_retry_max_retries_exceeded() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    let effect = retry(
      move || {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        Effect::new(async { Err::<i32, _>(IoError::new(ErrorKind::Other, "error")) })
      },
      2,
    );

    assert!(effect.run().await.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 3);
  }

  // Delay tests
  #[tokio::test]
  async fn test_delay_duration() {
    let start = Instant::now();
    let effect = delay(Effect::pure(42), Duration::from_millis(100));
    assert_eq!(effect.run().await, Ok(42));
    assert!(start.elapsed() >= Duration::from_millis(100));
  }

  #[tokio::test]
  async fn test_delay_with_error() {
    let effect = delay(
      Effect::new(async { Err::<i32, _>(IoError::new(ErrorKind::Other, "error")) }),
      Duration::from_millis(100),
    );
    assert!(effect.run().await.is_err());
  }

  #[tokio::test]
  async fn test_delay_zero_duration() {
    let effect = delay(Effect::pure(42), Duration::from_secs(0));
    assert_eq!(effect.run().await, Ok(42));
  }

  // Complex composition tests
  #[tokio::test]
  async fn test_complex_composition() {
    let effect = flat_map(Effect::pure(21), |x| {
      delay(Effect::pure(x * 2), Duration::from_millis(100))
    });
    assert_eq!(effect.run().await, Ok(42));
  }

  #[tokio::test]
  async fn test_retry_with_delay() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    let effect = retry(
      move || {
        let count = counter_clone.fetch_add(1, Ordering::SeqCst);
        delay(
          Effect::new(async move {
            if count < 2 {
              Err(IoError::new(ErrorKind::Other, "retry"))
            } else {
              Ok(42)
            }
          }),
          Duration::from_millis(50),
        )
      },
      3,
    );

    assert_eq!(effect.run().await, Ok(42));
    assert_eq!(counter.load(Ordering::SeqCst), 3);
  }

  #[tokio::test]
  async fn test_join_all() {
    let effects = vec![Effect::pure(1), Effect::pure(2), Effect::pure(3)];
    let result = join_all(effects).await.unwrap();
    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_race_ok() {
    let effects = vec![
      Effect::new(async {
        sleep(Duration::from_millis(100)).await;
        Ok(1)
      }),
      Effect::new(async {
        sleep(Duration::from_millis(50)).await;
        Ok(2)
      }),
      Effect::new(async {
        sleep(Duration::from_millis(150)).await;
        Ok(3)
      }),
    ];
    let result = race_ok(effects).await.unwrap();
    assert_eq!(result, 2);
  }

  #[tokio::test]
  async fn test_sequence() {
    let effects = vec![Effect::pure(1), Effect::pure(2), Effect::pure(3)];
    let result = sequence(effects).await.unwrap();
    assert_eq!(result, vec![1, 2, 3]);
  }
}
