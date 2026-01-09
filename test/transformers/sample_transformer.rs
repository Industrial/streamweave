//! Tests for SampleTransformer

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::transformers::SampleTransformer;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_sample_transformer_basic() {
  let mut transformer = SampleTransformer::new(1.0); // 100% probability
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // With 100% probability, should get all items
  assert_eq!(results.len(), 5);
}

#[tokio::test]
async fn test_sample_transformer_zero_probability() {
  let mut transformer = SampleTransformer::new(0.0); // 0% probability
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = transformer.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // With 0% probability, should get no items
  assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_sample_transformer_empty() {
  let mut transformer = SampleTransformer::new(0.5);
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = transformer.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_sample_transformer_with_name() {
  let transformer = SampleTransformer::new(0.5).with_name("sampler".to_string());

  let config = transformer.get_config_impl();
  assert_eq!(config.name(), Some("sampler"));
}

#[tokio::test]
async fn test_sample_transformer_with_error_strategy() {
  let transformer = SampleTransformer::new(0.5).with_error_strategy(ErrorStrategy::Skip);

  let config = transformer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[test]
#[should_panic(expected = "Probability must be between 0 and 1")]
fn test_sample_transformer_invalid_probability_high() {
  let _transformer = SampleTransformer::<i32>::new(1.5);
}

#[test]
#[should_panic(expected = "Probability must be between 0 and 1")]
fn test_sample_transformer_invalid_probability_negative() {
  let _transformer = SampleTransformer::<i32>::new(-0.1);
}
