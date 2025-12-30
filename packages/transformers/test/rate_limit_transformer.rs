use futures::StreamExt;
use futures::stream;
use std::time::Duration;
use streamweave_transformers::Input;
use streamweave_transformers::RateLimitTransformer;
use streamweave_transformers::Transformer;
use tokio::time::sleep;

#[cfg(test)]
mod tests {
  use super::*;
  use streamweave_error::ErrorStrategy;

  #[test]
  fn test_rate_limit_transformer_new() {
    let transformer = RateLimitTransformer::<i32>::new(10, Duration::from_secs(1));
    assert_eq!(transformer.rate_limit, 10);
    assert_eq!(transformer.time_window, Duration::from_secs(1));
    assert_eq!(
      transformer.count.load(std::sync::atomic::Ordering::Relaxed),
      0
    );
  }

  #[test]
  fn test_rate_limit_transformer_with_error_strategy() {
    let transformer = RateLimitTransformer::<i32>::new(5, Duration::from_secs(1))
      .with_error_strategy(ErrorStrategy::<i32>::Skip);
    assert!(matches!(
      transformer.config.error_strategy,
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_rate_limit_transformer_with_name() {
    let transformer = RateLimitTransformer::<i32>::new(5, Duration::from_secs(1))
      .with_name("test_rate_limit".to_string());
    assert_eq!(transformer.config.name, Some("test_rate_limit".to_string()));
  }

  #[tokio::test]
  async fn test_check_rate_limit_within_limit() {
    let transformer = RateLimitTransformer::<i32>::new(5, Duration::from_secs(1));
    let result = transformer._check_rate_limit().await;
    assert!(result.is_ok());
    assert_eq!(
      transformer.count.load(std::sync::atomic::Ordering::Relaxed),
      1
    );
  }

  #[tokio::test]
  async fn test_check_rate_limit_exceeded() {
    let transformer = RateLimitTransformer::<i32>::new(2, Duration::from_secs(1));

    // First two should succeed
    assert!(transformer._check_rate_limit().await.is_ok());
    assert!(transformer._check_rate_limit().await.is_ok());

    // Third should fail
    let result = transformer._check_rate_limit().await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_check_rate_limit_window_reset() {
    let transformer = RateLimitTransformer::<i32>::new(2, Duration::from_millis(100));

    // Use up the limit
    assert!(transformer._check_rate_limit().await.is_ok());
    assert!(transformer._check_rate_limit().await.is_ok());
    assert!(transformer._check_rate_limit().await.is_err());

    // Wait for window to reset
    sleep(Duration::from_millis(150)).await;

    // Should work again after window reset
    let result = transformer._check_rate_limit().await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_rate_limit_basic() {
    let mut transformer = RateLimitTransformer::new(3, Duration::from_millis(100));
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_rate_limit_empty_input() {
    let mut transformer = RateLimitTransformer::new(3, Duration::from_millis(100));
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_rate_limit_actual_rate_limit() {
    let mut transformer = RateLimitTransformer::new(2, Duration::from_millis(100));
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![1, 2]);
  }

  #[tokio::test]
  async fn test_rate_limit_reset() {
    let mut transformer = RateLimitTransformer::new(2, Duration::from_millis(100));
    let input = stream::iter(vec![1, 2]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![1, 2]);

    sleep(Duration::from_millis(200)).await;

    let input = stream::iter(vec![3, 4]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

    assert_eq!(result, vec![3, 4]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = RateLimitTransformer::new(3, Duration::from_millis(100))
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.get_config_impl();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
