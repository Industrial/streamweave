//! Batched inference transformer tests
//!
//! Tests for batched inference transformer that processes multiple items at once.

use streamweave::prelude::*;
use streamweave_ml::*;
use streamweave_vec::VecProducer;

// Mock backend for testing
#[derive(Clone)]
struct MockBackend {
  loaded: bool,
  multiplier: f32,
}

impl InferenceBackend for MockBackend {
  type Input = Vec<f32>;
  type Output = Vec<f32>;
  type Error = String;

  async fn load_from_path(&mut self, _path: &str) -> Result<(), Self::Error> {
    self.loaded = true;
    Ok(())
  }

  async fn load_from_bytes(&mut self, _bytes: &[u8]) -> Result<(), Self::Error> {
    self.loaded = true;
    Ok(())
  }

  async fn infer(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
    if !self.loaded {
      return Err("Model not loaded".to_string());
    }
    Ok(input.into_iter().map(|x| x * self.multiplier).collect())
  }

  async fn infer_batch(&self, inputs: Vec<Self::Input>) -> Result<Vec<Self::Output>, Self::Error> {
    if !self.loaded {
      return Err("Model not loaded".to_string());
    }
    Ok(
      inputs
        .into_iter()
        .map(|input| input.into_iter().map(|x| x * self.multiplier).collect())
        .collect(),
    )
  }

  fn is_loaded(&self) -> bool {
    self.loaded
  }

  fn input_shape(&self) -> Option<Vec<usize>> {
    Some(vec![2])
  }

  fn output_shape(&self) -> Option<Vec<usize>> {
    Some(vec![2])
  }
}

#[tokio::test]
async fn test_batched_inference_transformer_new() {
  let backend = MockBackend {
    loaded: true,
    multiplier: 2.0,
  };
  let transformer = BatchedInferenceTransformer::new(backend, 3); // batch size 3
  assert!(transformer.backend().read().await.is_loaded());
}

#[tokio::test]
async fn test_batched_inference_transformer_batching() {
  let backend = MockBackend {
    loaded: true,
    multiplier: 2.0,
  };
  let mut transformer = BatchedInferenceTransformer::new(backend, 3); // batch size 3

  let producer = VecProducer::new(vec![
    vec![1.0, 2.0],
    vec![3.0, 4.0],
    vec![5.0, 6.0],
    vec![7.0, 8.0],
  ]);
  let stream = producer.produce();

  let mut output = transformer.transform(Box::pin(stream)).await;
  let mut results = Vec::new();
  while let Some(item) = output.next().await {
    results.push(item);
  }

  // Should process in batches of 3, then 1
  // Results should be input * 2.0
  assert_eq!(results.len(), 4);
  assert_eq!(results[0], vec![2.0, 4.0]);
  assert_eq!(results[1], vec![6.0, 8.0]);
  assert_eq!(results[2], vec![10.0, 12.0]);
  assert_eq!(results[3], vec![14.0, 16.0]);
}

#[tokio::test]
async fn test_batched_inference_transformer_single_batch() {
  let backend = MockBackend {
    loaded: true,
    multiplier: 3.0,
  };
  let mut transformer = BatchedInferenceTransformer::new(backend, 10); // Large batch size

  let producer = VecProducer::new(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
  let stream = producer.produce();

  let mut output = transformer.transform(Box::pin(stream)).await;
  let mut results = Vec::new();
  while let Some(item) = output.next().await {
    results.push(item);
  }

  // Should process all in one batch
  assert_eq!(results.len(), 2);
  assert_eq!(results[0], vec![3.0, 6.0]);
  assert_eq!(results[1], vec![9.0, 12.0]);
}

#[tokio::test]
async fn test_batched_inference_transformer_empty_stream() {
  let backend = MockBackend {
    loaded: true,
    multiplier: 2.0,
  };
  let mut transformer = BatchedInferenceTransformer::new(backend, 3);

  let producer = VecProducer::new(vec![]);
  let stream = producer.produce();

  let mut output = transformer.transform(Box::pin(stream)).await;
  let mut results = Vec::new();
  while let Some(item) = output.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 0);
}
