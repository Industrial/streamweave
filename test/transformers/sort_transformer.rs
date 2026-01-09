//! Tests for SortTransformer

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::SortTransformer;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_sort_transformer_basic() {
  let mut transformer = SortTransformer::new();
  let input_stream = Box::pin(stream::iter(vec![3, 1, 4, 1, 5, 9, 2, 6]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 1, 2, 3, 4, 5, 6, 9]);
}

#[tokio::test]
async fn test_sort_transformer_already_sorted() {
  let mut transformer = SortTransformer::new();
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3, 4, 5]);
}

#[tokio::test]
async fn test_sort_transformer_reverse_order() {
  let mut transformer = SortTransformer::new();
  let input_stream = Box::pin(stream::iter(vec![5, 4, 3, 2, 1]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3, 4, 5]);
}

#[tokio::test]
async fn test_sort_transformer_empty() {
  let mut transformer = SortTransformer::new();
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_sort_transformer_single_item() {
  let mut transformer = SortTransformer::new();
  let input_stream = Box::pin(stream::iter(vec![42]));

  let mut output_stream = transformer.transform(input_stream).await;
  assert_eq!(output_stream.next().await, Some(42));
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_sort_transformer_default() {
  let transformer = SortTransformer::<i32>::default();
  let input_stream = Box::pin(stream::iter(vec![3, 1, 2]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_sort_transformer_with_name() {
  let transformer = SortTransformer::new().with_name("sorter".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("sorter"));
}

#[tokio::test]
async fn test_sort_transformer_with_error_strategy() {
  let transformer = SortTransformer::new().with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_sort_transformer_clone() {
  let transformer1 = SortTransformer::new();
  let transformer2 = transformer1.clone();

  let input_stream = Box::pin(stream::iter(vec![3, 1, 2]));
  let mut output_stream = transformer2.transform(input_stream).await;

  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3]);
}
