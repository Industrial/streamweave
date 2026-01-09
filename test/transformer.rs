//! Tests for transformer module

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::input::Input;
use streamweave::output::Output;
use streamweave::transformer::{Transformer, TransformerConfig, TransformerPorts};
use streamweave::transformers::MapTransformer;

#[test]
fn test_transformer_config_default() {
  let config = TransformerConfig::<i32>::default();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Stop));
  assert_eq!(config.name(), None);
}

#[test]
fn test_transformer_config_with_error_strategy() {
  let config = TransformerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[test]
fn test_transformer_config_with_name() {
  let config = TransformerConfig::<i32>::default().with_name("test".to_string());
  assert_eq!(config.name(), Some("test"));
}

#[test]
fn test_transformer_config_error_strategy() {
  let config = TransformerConfig::<i32>::default().with_error_strategy(ErrorStrategy::Retry(3));
  assert!(matches!(config.error_strategy(), ErrorStrategy::Retry(3)));
}

#[test]
fn test_transformer_config_name() {
  let config = TransformerConfig::<i32>::default().with_name("test".to_string());
  assert_eq!(config.name(), Some("test"));
}

#[test]
fn test_transformer_config_partial_eq() {
  let config1 = TransformerConfig::<i32>::default().with_name("test".to_string());
  let config2 = TransformerConfig::<i32>::default().with_name("test".to_string());
  assert_eq!(config1, config2);
}

#[tokio::test]
async fn test_transformer_trait_implementation() {
  let mut transformer = MapTransformer::new(|x: i32| x * 2);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![2, 4, 6]);
}

#[test]
fn test_transformer_ports_blanket_impl() {
  // Test that MapTransformer implements TransformerPorts
  type MapTransformerInputPorts =
    <MapTransformer<i32, i32, _> as TransformerPorts>::DefaultInputPorts;
  type MapTransformerOutputPorts =
    <MapTransformer<i32, i32, _> as TransformerPorts>::DefaultOutputPorts;
  // Should compile - this is a compile-time check
  assert!(true);
}
