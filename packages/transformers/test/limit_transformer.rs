use futures::{StreamExt, stream};
use proptest::prelude::*;
use streamweave::TransformerConfig;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_transformers::LimitTransformer;

#[tokio::test]
async fn test_limit_basic() {
  let mut transformer = LimitTransformer::new(3);
  let input = stream::iter(vec![1, 2, 3, 4, 5]);
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_limit_empty_input() {
  let mut transformer = LimitTransformer::new(3);
  let input = stream::iter(Vec::<i32>::new());
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, Vec::<i32>::new());
}

#[tokio::test]
async fn test_error_handling_strategies() {
  let transformer = LimitTransformer::new(3)
    .with_error_strategy(ErrorStrategy::<i32>::Skip)
    .with_name("test_transformer".to_string());

  let config = transformer.config();
  assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
  assert_eq!(config.name(), Some("test_transformer".to_string()));
}

#[tokio::test]
async fn test_limit_zero() {
  let mut transformer = LimitTransformer::new(0);
  let input = stream::iter(vec![1, 2, 3, 4, 5]);
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, Vec::<i32>::new());
}

#[tokio::test]
async fn test_limit_larger_than_input() {
  let mut transformer = LimitTransformer::new(10);
  let input = stream::iter(vec![1, 2, 3]);
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_limit_equal_to_input_size() {
  let mut transformer = LimitTransformer::new(3);
  let input = stream::iter(vec![1, 2, 3]);
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_limit_single_element() {
  let mut transformer = LimitTransformer::new(1);
  let input = stream::iter(vec![42]);
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, vec![42]);
}

#[tokio::test]
async fn test_limit_very_large() {
  let mut transformer = LimitTransformer::new(usize::MAX);
  let input = stream::iter(vec![1, 2, 3]);
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_limit_with_negative_numbers() {
  let mut transformer = LimitTransformer::new(2);
  let input = stream::iter(vec![-1, -2, -3, -4, -5]);
  let boxed_input = Box::pin(input);

  let result: Vec<i32> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, vec![-1, -2]);
}

#[tokio::test]
async fn test_limit_with_strings() {
  let mut transformer = LimitTransformer::new(2);
  let input = stream::iter(vec![
    "hello".to_string(),
    "world".to_string(),
    "test".to_string(),
  ]);
  let boxed_input = Box::pin(input);

  let result: Vec<String> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, vec!["hello".to_string(), "world".to_string()]);
}

#[tokio::test]
async fn test_limit_with_floats() {
  let mut transformer = LimitTransformer::new(3);
  let input = stream::iter(vec![1.1, 2.2, 3.3, 4.4, 5.5]);
  let boxed_input = Box::pin(input);

  let result: Vec<f64> = transformer.transform(boxed_input).await.collect().await;

  assert_eq!(result, vec![1.1, 2.2, 3.3]);
}

#[test]
fn test_limit_transformer_new() {
  let transformer = LimitTransformer::<i32>::new(5);
  assert_eq!(transformer.limit, 5);
}

#[test]
fn test_limit_transformer_with_error_strategy() {
  let transformer = LimitTransformer::<i32>::new(3).with_error_strategy(ErrorStrategy::<i32>::Skip);
  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[test]
fn test_limit_transformer_with_name() {
  let transformer = LimitTransformer::<i32>::new(3).with_name("test_limit".to_string());
  assert_eq!(transformer.config().name(), Some("test_limit".to_string()));
}

#[test]
fn test_limit_transformer_chaining() {
  let transformer = LimitTransformer::<i32>::new(10)
    .with_error_strategy(ErrorStrategy::<i32>::Retry(3))
    .with_name("chained_limit".to_string());

  assert_eq!(transformer.limit, 10);
  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Retry(3)
  ));
  assert_eq!(
    transformer.config().name(),
    Some("chained_limit".to_string())
  );
}

#[test]
fn test_limit_transformer_set_config_impl() {
  let mut transformer = LimitTransformer::<i32>::new(5);
  let new_config = TransformerConfig::default()
    .with_name("test_limit".to_string())
    .with_error_strategy(ErrorStrategy::Skip);
  transformer.set_config_impl(new_config);
  assert_eq!(transformer.config().name(), Some("test_limit".to_string()));
  assert!(matches!(
    transformer.config().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[test]
fn test_limit_transformer_get_config_impl() {
  let transformer = LimitTransformer::<i32>::new(5).with_name("test".to_string());
  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("test".to_string()));
}

#[test]
fn test_limit_transformer_get_config_mut_impl() {
  let mut transformer = LimitTransformer::<i32>::new(5);
  let config = transformer.get_config_mut_impl();
  config.name = Some("mutated".to_string());
  assert_eq!(transformer.config().name(), Some("mutated".to_string()));
}

#[test]
fn test_limit_transformer_handle_error_stop() {
  let transformer = LimitTransformer::<i32>::new(5).with_error_strategy(ErrorStrategy::Stop);
  let error = StreamError {
    source: Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "test")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "test".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "test".to_string(),
    },
    retries: 0,
  };
  assert_eq!(transformer.handle_error(&error), ErrorAction::Stop);
}

#[test]
fn test_limit_transformer_handle_error_skip() {
  let transformer = LimitTransformer::<i32>::new(5).with_error_strategy(ErrorStrategy::Skip);
  let error = StreamError {
    source: Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "test")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "test".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "test".to_string(),
    },
    retries: 0,
  };
  assert_eq!(transformer.handle_error(&error), ErrorAction::Skip);
}

#[test]
fn test_limit_transformer_handle_error_retry() {
  let transformer = LimitTransformer::<i32>::new(5).with_error_strategy(ErrorStrategy::Retry(3));
  let error = StreamError {
    source: Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "test")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "test".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "test".to_string(),
    },
    retries: 1,
  };
  assert_eq!(transformer.handle_error(&error), ErrorAction::Retry);
}

#[test]
fn test_limit_transformer_handle_error_retry_exhausted() {
  let transformer = LimitTransformer::<i32>::new(5).with_error_strategy(ErrorStrategy::Retry(3));
  let error = StreamError {
    source: Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "test")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "test".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "test".to_string(),
    },
    retries: 3,
  };
  assert_eq!(transformer.handle_error(&error), ErrorAction::Stop);
}

#[test]
fn test_limit_transformer_create_error_context() {
  let transformer = LimitTransformer::<i32>::new(5).with_name("test_limit".to_string());
  let context = transformer.create_error_context(Some(42));
  assert_eq!(context.component_name, "test_limit");
  assert_eq!(context.item, Some(42));
  assert!(context.component_type.contains("LimitTransformer"));
}

#[test]
fn test_limit_transformer_create_error_context_no_item() {
  let transformer = LimitTransformer::<i32>::new(5);
  let context = transformer.create_error_context(None);
  assert_eq!(context.component_name, "limit_transformer");
  assert_eq!(context.item, None);
}

#[test]
fn test_limit_transformer_component_info() {
  let transformer = LimitTransformer::<i32>::new(5).with_name("custom_limit".to_string());
  let info = transformer.component_info();
  assert_eq!(info.name, "custom_limit");
  assert!(info.type_name.contains("LimitTransformer"));
}

#[test]
fn test_limit_transformer_component_info_default() {
  let transformer = LimitTransformer::<i32>::new(5);
  let info = transformer.component_info();
  assert_eq!(info.name, "limit_transformer");
}
