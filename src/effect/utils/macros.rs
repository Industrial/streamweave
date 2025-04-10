//! Effect macros implementation.
//!
//! This module provides macros for working with effects.

use effect_core::effect::Effect;

/// Creates a new effect from a value.
#[macro_export]
macro_rules! effect {
  ($value:expr) => {
    Effect::pure($value)
  };
}

/// Creates a new effect from a result.
#[macro_export]
macro_rules! try_effect {
  ($result:expr) => {
    Effect::new(async move { $result })
  };
}

/// Creates a new effect that runs multiple effects in parallel.
#[macro_export]
macro_rules! parallel {
    ($($effect:expr),* $(,)?) => {{
        let effects = vec![$($effect),*];
        crate::effect::concurrent::structured::parallel(effects)
    }};
}

/// Creates a new effect that runs multiple effects in sequence.
#[macro_export]
macro_rules! sequence {
    ($($effect:expr),* $(,)?) => {{
        let effects = vec![$($effect),*];
        crate::effect::concurrent::structured::sequence(effects)
    }};
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::effect::core::effect::Effect;
  use std::error::Error as StdError;
  use std::io::{Error as IoError, ErrorKind};
  use std::time::Duration;
  use tokio::time::sleep;

  // Basic effect macro tests
  #[tokio::test]
  async fn test_effect_basic() {
    let effect = effect!(42);
    assert_eq!(effect.run().await.unwrap(), 42);

    let effect = effect!("hello".to_string());
    assert_eq!(effect.run().await.unwrap(), "hello".to_string());

    let effect = effect!(vec![1, 2, 3]);
    assert_eq!(effect.run().await.unwrap(), vec![1, 2, 3]);
  }

  // Try effect macro tests
  #[tokio::test]
  async fn test_try_effect_success() {
    let effect = try_effect!(Ok::<_, IoError>(42));
    assert_eq!(effect.run().await.unwrap(), 42);

    let effect = try_effect!(Ok::<String, IoError>("hello".to_string()));
    assert_eq!(effect.run().await.unwrap(), "hello".to_string());
  }

  #[tokio::test]
  async fn test_try_effect_error() {
    let effect = try_effect!(Err::<i32, _>(IoError::new(ErrorKind::Other, "test error")));
    let result = effect.run().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "test error");
  }

  // Parallel macro tests
  #[tokio::test]
  async fn test_parallel_basic() {
    let effect = parallel!(
      Effect::new(async { Ok::<_, IoError>(1) }),
      Effect::new(async { Ok(2) }),
      Effect::new(async { Ok(3) })
    );
    let result = effect.run().await.unwrap();
    assert_eq!(result.len(), 3);
    assert!(result.contains(&1));
    assert!(result.contains(&2));
    assert!(result.contains(&3));
  }

  #[tokio::test]
  async fn test_parallel_empty() {
    let effect: Effect<Vec<i32>, IoError> = parallel!();
    let result = effect.run().await.unwrap();
    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_parallel_single() {
    let effect = parallel!(Effect::new(async { Ok::<_, IoError>(42) }));
    let result = effect.run().await.unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], 42);
  }

  #[tokio::test]
  async fn test_parallel_with_delays() {
    let effect1 = Effect::new(async {
      sleep(Duration::from_millis(50)).await;
      Ok::<_, IoError>(1)
    });
    let effect2 = Effect::new(async {
      sleep(Duration::from_millis(10)).await;
      Ok(2)
    });
    let effect3 = Effect::new(async {
      sleep(Duration::from_millis(30)).await;
      Ok(3)
    });

    let effect = parallel!(effect1, effect2, effect3);
    let result = effect.run().await.unwrap();
    assert_eq!(result.len(), 3);
    assert!(result.contains(&1));
    assert!(result.contains(&2));
    assert!(result.contains(&3));
  }

  #[tokio::test]
  async fn test_parallel_with_error() {
    let effect1 = Effect::new(async { Ok::<_, IoError>(1) });
    let effect2 = Effect::new(async { Err(IoError::new(ErrorKind::Other, "test error")) });
    let effect3 = Effect::new(async { Ok(3) });

    let effect = parallel!(effect1, effect2, effect3);
    let result = effect.run().await;
    assert!(result.is_err());
  }

  // Sequence macro tests
  #[tokio::test]
  async fn test_sequence_basic() {
    let effect = sequence!(
      Effect::new(async { Ok::<_, IoError>(1) }),
      Effect::new(async { Ok(2) }),
      Effect::new(async { Ok(3) })
    );
    let result = effect.run().await.unwrap();
    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_sequence_empty() {
    let effect: Effect<Vec<i32>, IoError> = sequence!();
    let result = effect.run().await.unwrap();
    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_sequence_single() {
    let effect = sequence!(Effect::new(async { Ok::<_, IoError>(42) }));
    let result = effect.run().await.unwrap();
    assert_eq!(result, vec![42]);
  }

  #[tokio::test]
  async fn test_sequence_with_delays() {
    let effect1 = Effect::new(async {
      sleep(Duration::from_millis(50)).await;
      Ok::<_, IoError>(1)
    });
    let effect2 = Effect::new(async {
      sleep(Duration::from_millis(10)).await;
      Ok(2)
    });
    let effect3 = Effect::new(async {
      sleep(Duration::from_millis(30)).await;
      Ok(3)
    });

    let effect = sequence!(effect1, effect2, effect3);
    let result = effect.run().await.unwrap();
    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_sequence_with_error() {
    let effect1 = Effect::new(async { Ok::<_, IoError>(1) });
    let effect2 = Effect::new(async { Err(IoError::new(ErrorKind::Other, "test error")) });
    let effect3 = Effect::new(async { Ok(3) });

    let effect = sequence!(effect1, effect2, effect3);
    let result = effect.run().await;
    assert!(result.is_err());
  }

  // Mixed macro usage tests
  #[tokio::test]
  async fn test_mixed_macros() {
    let parallel_effect = parallel!(
      Effect::new(async { Ok::<_, IoError>(1) }),
      Effect::new(async { Ok(2) })
    );
    let sequence_effect = sequence!(Effect::new(async { Ok(3) }), Effect::new(async { Ok(4) }));
    let combined = sequence!(parallel_effect, sequence_effect);

    let result = combined.run().await.unwrap();
    assert_eq!(result.len(), 2);
    assert!(result[0].contains(&1));
    assert!(result[0].contains(&2));
    assert_eq!(result[1], vec![3, 4]);
  }

  // Different type tests
  #[tokio::test]
  async fn test_different_types() {
    let effect = parallel!(
      Effect::new(async { Ok::<_, IoError>("hello".to_string()) }),
      Effect::new(async { Ok(42) }),
      Effect::new(async { Ok(vec![1, 2, 3]) })
    );
    let result = effect.run().await.unwrap();
    assert_eq!(result.len(), 3);
  }
}
