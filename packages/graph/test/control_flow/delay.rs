use futures::{StreamExt, stream};
use proptest::prelude::*;
use std::pin::Pin;
use std::time::Duration;
use streamweave::Transformer;
use streamweave_graph::control_flow::Delay;

#[tokio::test]
async fn test_delay_transformer() {
  let delay_duration = Duration::from_millis(50);
  let mut transformer = Delay::new(delay_duration);
  let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 2, 3]));

  let start = std::time::Instant::now();
  let output_stream = transformer.transform(input_stream).await;
  let results: Vec<i32> = output_stream.collect().await;
  let elapsed = start.elapsed();

  assert_eq!(results, vec![1, 2, 3]);
  // Should have at least 2 delays (between items)
  assert!(elapsed >= delay_duration * 2);
}

proptest! {
  #![proptest_config(ProptestConfig::with_cases(10))]

  #[test]
  fn test_delay_proptest(numbers in prop::collection::vec(-1000i32..1000, 0..10), delay_ms in 0u64..=5u64) {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async {
      let mut delay = Delay::new(Duration::from_millis(delay_ms));
      let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
        Box::pin(stream::iter(numbers.clone()));

      let start = std::time::Instant::now();
      let mut output = delay.transform(input_stream).await;
      let mut results = Vec::new();
      while let Some(item) = output.next().await {
        results.push(item);
      }
      let elapsed = start.elapsed();

      // Verify all items are preserved
      assert_eq!(results, numbers);

      // Verify delay was applied (at least minimum delay)
      // For very small delays, we allow some tolerance due to timing precision
      if !numbers.is_empty() && delay_ms > 0 {
        let min_expected = Duration::from_millis(delay_ms * numbers.len() as u64);
        // Allow 10% tolerance for timing precision
        let tolerance = min_expected / 10;
        assert!(elapsed >= min_expected.saturating_sub(tolerance),
                "Expected at least {:?}, but got {:?}", min_expected, elapsed);
      }
    });
  }
}
