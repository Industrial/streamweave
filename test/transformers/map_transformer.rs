//! Tests for MapTransformer

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::MapTransformer;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_map_transformer_basic() {
  let mut transformer = MapTransformer::new(|x: i32| x * 2);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![2, 4, 6, 8, 10]);
}

#[tokio::test]
async fn test_map_transformer_empty() {
  let mut transformer = MapTransformer::new(|x: i32| x * 2);
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_map_transformer_type_conversion() {
  let mut transformer = MapTransformer::new(|x: i32| x.to_string());
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(
    results,
    vec!["1".to_string(), "2".to_string(), "3".to_string()]
  );
}

#[tokio::test]
async fn test_map_transformer_with_name() {
  let transformer = MapTransformer::new(|x: i32| x * 2).with_name("doubler".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("doubler"));
}

#[tokio::test]
async fn test_map_transformer_with_error_strategy() {
  let transformer = MapTransformer::new(|x: i32| x * 2).with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy, ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_map_transformer_config_operations() {
  let mut transformer = MapTransformer::new(|x: i32| x * 2);

  let config = transformer.get_config_mut_impl();
  config.name = Some("updated".to_string());

  assert_eq!(transformer.get_config_impl().name(), Some("updated"));
}

#[tokio::test]
async fn test_map_transformer_clone() {
  let transformer1 = MapTransformer::new(|x: i32| x * 2);
  let transformer2 = transformer1.clone();

  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));
  let mut output_stream1 = transformer1.transform(input_stream).await;

  let input_stream2 = Box::pin(stream::iter(vec![1, 2, 3]));
  let mut output_stream2 = transformer2.transform(input_stream2).await;

  assert_eq!(output_stream1.next().await, Some(2));
  assert_eq!(output_stream2.next().await, Some(2));
}

#[tokio::test]
async fn test_map_transformer_component_info() {
  let transformer = MapTransformer::new(|x: i32| x * 2).with_name("test".to_string());

  let info = transformer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("MapTransformer"));
}
