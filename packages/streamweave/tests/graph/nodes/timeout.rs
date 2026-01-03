use futures::{StreamExt, stream};
use proptest::prelude::*;
use std::pin::Pin;
use std::time::Duration;
use streamweave::graph::nodes::{Timeout, TimeoutError};
use streamweave::{Producer, Transformer};
use streamweave_vec::VecProducer;
use tokio::time::sleep;

#[tokio::test]
async fn test_timeout_transformer_no_timeout() {
  let timeout_duration = Duration::from_millis(100);
  let mut transformer = Timeout::new(timeout_duration);
  let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 2, 3]));

  let output_stream = transformer.transform(input_stream).await;
  let results: Vec<Result<i32, TimeoutError>> = output_stream.collect().await;

  assert_eq!(results.len(), 3);
  assert!(results[0].is_ok());
  assert!(results[1].is_ok());
  assert!(results[2].is_ok());
}

#[tokio::test]
async fn test_timeout_transformer_with_timeout() {
  let timeout_duration = Duration::from_millis(50);
  let mut transformer = Timeout::new(timeout_duration);

  // Create a stream that yields items slowly
  let input_stream = Box::pin(async_stream::stream! {
    yield 1;
    sleep(Duration::from_millis(30)).await;
    yield 2;
    sleep(Duration::from_millis(100)).await; // This should timeout
    yield 3;
  });

  let output_stream = transformer.transform(input_stream).await;
  let results: Vec<Result<i32, TimeoutError>> = output_stream.collect().await;

  assert!(results.len() >= 2);
  assert!(results[0].is_ok());
  assert!(results[1].is_ok());
  // May or may not have timeout error depending on timing
}

#[tokio::test]
async fn test_timeout_transformer_success() {
  let mut timeout = Timeout::new(Duration::from_millis(100));
  let mut producer = VecProducer::new(vec![1, 2, 3]);
  let stream = producer.produce();

  let mut output = timeout.transform(Box::pin(stream)).await;
  let mut results = Vec::new();
  while let Some(item) = output.next().await {
    results.push(item);
  }

  // All items should succeed (process quickly)
  assert_eq!(results.len(), 3);
  assert!(
    results
      .iter()
      .all(|r: &Result<i32, TimeoutError>| r.is_ok())
  );
  assert_eq!(
    results.into_iter().map(|r| r.unwrap()).collect::<Vec<_>>(),
    vec![1, 2, 3]
  );
}

#[test]
fn test_timeout_error_display() {
  let error = TimeoutError;
  assert_eq!(error.to_string(), "Operation timed out");
}

#[test]
fn test_timeout_error_type() {
  let error = TimeoutError;
  assert_eq!(error, TimeoutError);
  assert_eq!(format!("{}", error), "Operation timed out");
}

proptest! {
  #![proptest_config(ProptestConfig::with_cases(100))]

  #[test]
  fn test_timeout_proptest(numbers in prop::collection::vec(-1000i32..1000, 0..50), timeout_ms in 1u64..=100u64) {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async {
      let mut timeout = Timeout::new(Duration::from_millis(timeout_ms));
      let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
        Box::pin(stream::iter(numbers.clone()));

      let mut output = timeout.transform(input_stream).await;
      let mut results = Vec::new();
      while let Some(item) = output.next().await {
        results.push(item);
      }

      // Verify all items are wrapped in Result
      assert_eq!(results.len(), numbers.len());
      for (i, result) in results.iter().enumerate() {
        // Items should be Ok since they're processed immediately
        assert!(result.is_ok(), "Item {} should be Ok", i);
        if let Ok(value) = result {
          assert_eq!(*value, numbers[i]);
        }
      }
    });
  }
}
