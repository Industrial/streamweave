//! Tests for FilterTransformer

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::FilterTransformer;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_filter_transformer_basic() {
  let mut transformer = FilterTransformer::new(|x: &i32| *x > 2);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![3, 4, 5]);
}

#[tokio::test]
async fn test_filter_transformer_all_pass() {
  let mut transformer = FilterTransformer::new(|x: &i32| *x > 0);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_filter_transformer_none_pass() {
  let mut transformer = FilterTransformer::new(|x: &i32| *x > 100);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_filter_transformer_empty() {
  let mut transformer = FilterTransformer::new(|x: &i32| *x > 2);
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_filter_transformer_with_name() {
  let transformer = FilterTransformer::new(|x: &i32| *x > 2).with_name("filter_gt_2".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("filter_gt_2"));
}

#[tokio::test]
async fn test_filter_transformer_with_error_strategy() {
  let transformer =
    FilterTransformer::new(|x: &i32| *x > 2).with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy, ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_filter_transformer_component_info() {
  let transformer = FilterTransformer::new(|x: &i32| *x > 2).with_name("test".to_string());

  let info = transformer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("FilterTransformer"));
}
