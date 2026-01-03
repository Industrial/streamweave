//! Tests for RouterTransformer

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::{RouteTarget, RouterTransformer};
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_route_target_variants() {
  assert_eq!(
    RouteTarget::named("test"),
    RouteTarget::Named("test".to_string())
  );
  assert_eq!(RouteTarget::index(5), RouteTarget::Index(5));
  assert_eq!(RouteTarget::default_route(), RouteTarget::Default);
  assert_eq!(RouteTarget::drop(), RouteTarget::Drop);
}

#[tokio::test]
async fn test_router_transformer_basic() {
  let mut transformer = RouterTransformer::new(|x: &i32| {
    if *x > 5 {
      RouteTarget::named("high")
    } else {
      RouteTarget::named("low")
    }
  });
  let input_stream = Box::pin(stream::iter(vec![1, 6, 2, 7]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should route items based on predicate
  assert_eq!(results.len(), 4);
}

#[tokio::test]
async fn test_router_transformer_drop() {
  let mut transformer = RouterTransformer::new(|x: &i32| {
    if *x > 5 {
      RouteTarget::drop()
    } else {
      RouteTarget::default_route()
    }
  });
  let input_stream = Box::pin(stream::iter(vec![1, 6, 2, 7]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Dropped items should not appear
  assert_eq!(results.len(), 2); // Only 1 and 2
}

#[tokio::test]
async fn test_router_transformer_empty() {
  let mut transformer = RouterTransformer::new(|x: &i32| RouteTarget::default_route());
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_router_transformer_with_name() {
  let transformer =
    RouterTransformer::new(|x: &i32| RouteTarget::default_route()).with_name("router".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("router"));
}

#[tokio::test]
async fn test_router_transformer_with_error_strategy() {
  let transformer = RouterTransformer::new(|x: &i32| RouteTarget::default_route())
    .with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_router_transformer_clone() {
  let transformer1 = RouterTransformer::new(|x: &i32| RouteTarget::default_route());
  let transformer2 = transformer1.clone();

  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));
  let mut output_stream = transformer2.transform(input_stream).await;

  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 3);
}
