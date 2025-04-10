//! Effect cancellation implementation.
//!
//! This module provides types and traits for cancelling effects, including
//! cancellation tokens and cancellable effects.

use std::convert::From;
use std::error::Error as StdError;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use effect_core::effect::Effect;
use effect_core::monad::Monad;

/// A token that can be used to cancel an effect.
#[derive(Clone)]
pub struct CancellationToken {
  cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
  /// Creates a new cancellation token.
  pub fn new() -> Self {
    Self {
      cancelled: Arc::new(AtomicBool::new(false)),
    }
  }

  /// Returns true if the token has been cancelled.
  pub fn is_cancelled(&self) -> bool {
    self.cancelled.load(Ordering::SeqCst)
  }

  /// Cancels the token.
  pub fn cancel(&self) {
    self.cancelled.store(true, Ordering::SeqCst);
  }
}

impl Default for CancellationToken {
  fn default() -> Self {
    Self::new()
  }
}

/// A trait for effects that can be cancelled.
pub trait Cancellable<T, E: StdError + Send + Sync + 'static> {
  /// Makes this effect cancellable with the given token.
  fn with_cancellation(self, token: CancellationToken) -> Effect<T, E>;
}

impl<T, E> Cancellable<T, E> for Effect<T, E>
where
  T: Send + Sync + 'static,
  E: StdError + Send + Sync + 'static + From<std::io::Error>,
{
  fn with_cancellation(self, token: CancellationToken) -> Effect<T, E> {
    Effect::new(async move {
      let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));

      tokio::select! {
          result = self.run() => result,
          _ = async move {
              while !token.is_cancelled() {
                  interval.tick().await;
              }
          } => Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "Effect cancelled").into()),
      }
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::{Error as IoError, ErrorKind};
  use std::time::Duration;
  use tokio::time::sleep;

  // Test immediate cancellation
  #[tokio::test]
  async fn test_immediate_cancellation() {
    let token = CancellationToken::new();
    token.cancel();

    let effect: Effect<i32, IoError> = Effect::pure(42).with_cancellation(token);
    let result = effect.run().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), ErrorKind::Interrupted);
  }

  // Test delayed cancellation
  #[tokio::test]
  async fn test_delayed_cancellation() {
    let token = CancellationToken::new();
    let token_clone = token.clone();

    let effect: Effect<i32, IoError> = Effect::new(async move {
      sleep(Duration::from_secs(1)).await;
      Ok(42)
    })
    .with_cancellation(token);

    tokio::spawn(async move {
      sleep(Duration::from_millis(100)).await;
      token_clone.cancel();
    });

    let result = effect.run().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), ErrorKind::Interrupted);
  }

  // Test successful completion before cancellation
  #[tokio::test]
  async fn test_completion_before_cancellation() {
    let token = CancellationToken::new();
    let token_clone = token.clone();

    let effect: Effect<i32, IoError> = Effect::new(async move {
      sleep(Duration::from_millis(50)).await;
      Ok(42)
    })
    .with_cancellation(token);

    tokio::spawn(async move {
      sleep(Duration::from_millis(100)).await;
      token_clone.cancel();
    });

    assert_eq!(effect.run().await, Ok(42));
  }

  // Test cancellation with chained effects
  #[tokio::test]
  async fn test_chained_effects_cancellation() {
    let token = CancellationToken::new();
    let token_clone = token.clone();

    let effect: Effect<String, IoError> = Effect::pure(42)
      .map(|x| x.to_string())
      .with_cancellation(token);

    tokio::spawn(async move {
      sleep(Duration::from_millis(50)).await;
      token_clone.cancel();
    });

    assert!(effect.run().await.is_err());
  }

  // Test multiple cancellation tokens
  #[tokio::test]
  async fn test_multiple_tokens() {
    let token1 = CancellationToken::new();
    let token2 = CancellationToken::new();

    let effect: Effect<i32, IoError> = Effect::pure(42)
      .with_cancellation(token1.clone())
      .with_cancellation(token2.clone());

    // Cancel only the second token
    token2.cancel();
    assert!(effect.run().await.is_err());

    // Create new effect with fresh tokens
    let token3 = CancellationToken::new();
    let token4 = CancellationToken::new();

    let effect: Effect<i32, IoError> = Effect::pure(42)
      .with_cancellation(token3.clone())
      .with_cancellation(token4.clone());

    // Cancel only the first token
    token3.cancel();
    assert!(effect.run().await.is_err());
  }

  // Test cancellation with complex async operations
  #[tokio::test]
  async fn test_complex_async_cancellation() {
    let token = CancellationToken::new();
    let token_clone = token.clone();

    let effect: Effect<Vec<i32>, IoError> = Effect::new(async {
      let mut results = Vec::new();
      for i in 0..5 {
        sleep(Duration::from_millis(50)).await;
        results.push(i);
      }
      Ok(results)
    })
    .with_cancellation(token);

    tokio::spawn(async move {
      sleep(Duration::from_millis(120)).await;
      token_clone.cancel();
    });

    let result = effect.run().await;
    assert!(result.is_err());
  }

  // Test cancellation token reuse
  #[tokio::test]
  async fn test_token_reuse() {
    let token = CancellationToken::new();

    // First effect
    let effect1: Effect<i32, IoError> = Effect::pure(42).with_cancellation(token.clone());
    assert_eq!(effect1.run().await, Ok(42));

    // Cancel token
    token.cancel();

    // Second effect with same token
    let effect2: Effect<i32, IoError> = Effect::pure(42).with_cancellation(token.clone());
    assert!(effect2.run().await.is_err());
  }

  // Test error propagation
  #[tokio::test]
  async fn test_error_propagation() {
    let token = CancellationToken::new();

    let effect: Effect<i32, IoError> =
      Effect::new(async { Err(IoError::new(ErrorKind::Other, "original error")) })
        .with_cancellation(token.clone());

    // Error should propagate without cancellation
    let result = effect.run().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), ErrorKind::Other);

    // Now with cancellation
    token.cancel();
    let effect: Effect<i32, IoError> = Effect::new(async {
      sleep(Duration::from_millis(100)).await;
      Err(IoError::new(ErrorKind::Other, "original error"))
    })
    .with_cancellation(token);

    let result = effect.run().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), ErrorKind::Interrupted);
  }
}
