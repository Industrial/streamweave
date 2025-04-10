//! Effect combinators implementation.
//!
//! This module provides combinators for composing and transforming effects.

use std::error::Error as StdError;
use std::future::Future;

use effect_core::effect::Effect;
use std::time::{Duration, Instant};
use tokio::time::sleep;

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
  use std::io::{Error, ErrorKind};
  use std::sync::Arc;
  use std::sync::atomic::{AtomicUsize, Ordering};

  #[tokio::test]
  async fn test_monad_laws() {
    // Left identity: pure(a).flat_map(f) == f(a)
    let a = 42;
    let f = |x: i32| Effect::<i32, Error>::pure(x * 2);
    let left = Effect::<i32, Error>::pure(a).flat_map(f);
    let right = f(a);
    assert_eq!(left.run().await.unwrap(), right.run().await.unwrap());

    // Right identity: m.flat_map(pure) == m
    let m = Effect::<i32, Error>::pure(42);
    let pure = |x: i32| Effect::<i32, Error>::pure(x);
    assert_eq!(
      m.flat_map(pure).run().await.unwrap(),
      m.run().await.unwrap()
    );

    // Associativity: m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
    let m = Effect::<i32, Error>::pure(42);
    let f = |x: i32| Effect::<i32, Error>::pure(x * 2);
    let g = |x: i32| Effect::<i32, Error>::pure(x + 1);
    let left = m.flat_map(f).flat_map(g);
    let right = m.flat_map(|x| f(x).flat_map(g));
    assert_eq!(left.run().await.unwrap(), right.run().await.unwrap());
  }

  #[tokio::test]
  async fn test_error_handling() {
    let effect =
      Effect::<i32, Error>::new(async move { Err(Error::new(ErrorKind::Other, "test error")) });

    let result = effect.run().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), ErrorKind::Other);
  }

  #[tokio::test]
  async fn test_async_behavior() {
    let effect = Effect::<i32, Error>::new(async move {
      sleep(Duration::from_millis(100)).await;
      Ok(42)
    });

    let start = Instant::now();
    let result = effect.run().await;
    let duration = start.elapsed();

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
    assert!(duration >= Duration::from_millis(100));
  }

  #[tokio::test]
  async fn test_edge_cases() {
    // Test empty effect
    let empty = Effect::<(), Error>::pure(());
    assert!(empty.run().await.is_ok());

    // Test unit type
    let unit = Effect::<(), Error>::pure(());
    assert!(unit.run().await.is_ok());

    // Test complex type
    let complex = Effect::<Vec<i32>, Error>::pure(vec![1, 2, 3]);
    assert_eq!(complex.run().await.unwrap(), vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_resource_management() {
    let mut resource = 0;
    let effect = Effect::<i32, Error>::new(async move {
      resource += 1;
      Ok(resource)
    });

    let result = effect.run().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);
  }

  #[tokio::test]
  async fn test_combinators() {
    let effect = Effect::<i32, Error>::pure(42)
      .map(|x| x * 2)
      .flat_map(|x| Effect::pure(x + 1));

    assert_eq!(effect.run().await.unwrap(), 85);
  }

  #[tokio::test]
  async fn test_type_safety() {
    let effect: Effect<i32, Error> = Effect::pure(42);
    let mapped: Effect<String, Error> = effect.map(|x| x.to_string());
    assert_eq!(mapped.run().await.unwrap(), "42");
  }

  #[tokio::test]
  async fn test_complex_composition() {
    let effect = Effect::<i32, Error>::pure(42)
      .flat_map(|x| Effect::pure(x * 2))
      .flat_map(|x| Effect::pure(x + 1))
      .map(|x| x.to_string());

    assert_eq!(effect.run().await.unwrap(), "85");
  }

  #[tokio::test]
  async fn test_error_recovery() {
    let effect =
      Effect::<i32, Error>::new(async move { Err(Error::new(ErrorKind::Other, "test error")) })
        .flat_map(|_| Effect::pure(42));

    assert!(effect.run().await.is_err());
  }

  #[tokio::test]
  async fn test_timing_guarantees() {
    let effect = Effect::<i32, Error>::new(async move {
      sleep(Duration::from_millis(200)).await;
      Ok(42)
    });

    let start = Instant::now();
    let _ = effect.run().await;
    let duration = start.elapsed();

    assert!(duration >= Duration::from_millis(200));
  }

  #[tokio::test]
  async fn test_future_conversion() {
    let effect = Effect::<i32, Error>::pure(42);
    let future = effect.into_future();
    assert_eq!(future.await.unwrap(), 42);
  }
}
