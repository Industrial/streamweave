//! Effect supervision implementation.
//!
//! This module provides types and traits for supervising concurrent effects,
//! including different supervision strategies and error handling.

use std::error::Error as StdError;
use std::future::Future;

use crate::effect::core::effect::Effect;

/// The strategy to use when an effect fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorStrategy {
  /// Stop all effects when one fails.
  StopOnFailure,
  /// Continue running other effects when one fails.
  ContinueOnFailure,
  /// Retry failed effects up to a maximum number of times.
  Retry { max_retries: usize },
}

/// A trait for supervising concurrent effects.
pub trait Supervisor<T, E: StdError + Send + Sync + 'static> {
  /// Supervises a collection of effects with the given strategy.
  fn supervise<I>(effects: I, strategy: SupervisorStrategy) -> Effect<Vec<Result<T, E>>, E>
  where
    I: IntoIterator<Item = Effect<T, E>>,
    I::IntoIter: Send + 'static;
}

impl<T, E> Supervisor<T, E> for Effect<T, E>
where
  T: Send + Sync + 'static,
  E: StdError + Send + Sync + 'static,
{
  fn supervise<I>(effects: I, strategy: SupervisorStrategy) -> Effect<Vec<Result<T, E>>, E>
  where
    I: IntoIterator<Item = Effect<T, E>>,
    I::IntoIter: Send + 'static,
  {
    Effect::new(async move {
      let futures: Vec<_> = effects.into_iter().map(|effect| effect.run()).collect();

      let mut results = Vec::with_capacity(futures.len());
      let mut any_failed = false;

      for future in futures {
        match strategy {
          SupervisorStrategy::StopOnFailure => {
            let result = future.await;
            results.push(result.clone());
            if result.is_err() {
              return Ok(results);
            }
          }
          SupervisorStrategy::ContinueOnFailure => {
            results.push(future.await);
          }
          SupervisorStrategy::Retry { max_retries } => {
            let mut retries = 0;
            let mut result = future.await;
            while result.is_err() && retries < max_retries {
              result = future.await;
              retries += 1;
            }
            results.push(result);
          }
        }
      }

      Ok(results)
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::{Error as IoError, ErrorKind};

  #[tokio::test]
  async fn test_stop_on_failure() {
    let effects = vec![
      Effect::pure(1),
      Effect::new(async { Err(IoError::new(ErrorKind::Other, "test error")) }),
      Effect::pure(3),
    ];

    let results = Effect::supervise(effects, SupervisorStrategy::StopOnFailure)
      .run()
      .await
      .unwrap();

    assert_eq!(results.len(), 2);
    assert!(results[0].is_ok());
    assert!(results[1].is_err());
  }

  #[tokio::test]
  async fn test_continue_on_failure() {
    let effects = vec![
      Effect::pure(1),
      Effect::new(async { Err(IoError::new(ErrorKind::Other, "test error")) }),
      Effect::pure(3),
    ];

    let results = Effect::supervise(effects, SupervisorStrategy::ContinueOnFailure)
      .run()
      .await
      .unwrap();

    assert_eq!(results.len(), 3);
    assert!(results[0].is_ok());
    assert!(results[1].is_err());
    assert!(results[2].is_ok());
  }

  #[tokio::test]
  async fn test_retry() {
    let mut retry_count = 0;
    let effects = vec![
      Effect::pure(1),
      Effect::new(async move {
        retry_count += 1;
        if retry_count < 3 {
          Err(IoError::new(ErrorKind::Other, "test error"))
        } else {
          Ok(2)
        }
      }),
      Effect::pure(3),
    ];

    let results = Effect::supervise(effects, SupervisorStrategy::Retry { max_retries: 3 })
      .run()
      .await
      .unwrap();

    assert_eq!(results.len(), 3);
    assert!(results[0].is_ok());
    assert!(results[1].is_ok());
    assert!(results[2].is_ok());
  }

  #[tokio::test]
  async fn test_retry_max_exceeded() {
    let effects = vec![
      Effect::pure(1),
      Effect::new(async { Err(IoError::new(ErrorKind::Other, "test error")) }),
      Effect::pure(3),
    ];

    let results = Effect::supervise(effects, SupervisorStrategy::Retry { max_retries: 1 })
      .run()
      .await
      .unwrap();

    assert_eq!(results.len(), 3);
    assert!(results[0].is_ok());
    assert!(results[1].is_err());
    assert!(results[2].is_ok());
  }
}
