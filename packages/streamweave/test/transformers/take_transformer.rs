//! Tests for TakeTransformer

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::TakeTransformer;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_take_transformer_basic() {
  let mut transformer = TakeTransformer::new(3);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_take_transformer_more_than_available() {
  let mut transformer = TakeTransformer::new(10);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_take_transformer_zero() {
  let mut transformer = TakeTransformer::new(0);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_take_transformer_empty() {
  let mut transformer = TakeTransformer::new(3);
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_take_transformer_with_name() {
  let transformer = TakeTransformer::new(5).with_name("take_5".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("take_5"));
}

#[tokio::test]
async fn test_take_transformer_with_error_strategy() {
  let transformer = TakeTransformer::new(5).with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_take_transformer_clone() {
  let transformer1 = TakeTransformer::new(3);
  let transformer2 = transformer1.clone();

  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));
  let mut output_stream = transformer2.transform(input_stream).await;

  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3]);
}
