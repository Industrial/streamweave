use futures::{StreamExt, stream};
use proptest::prelude::*;
use std::pin::Pin;
use streamweave::{Producer, Transformer};
use streamweave_graph::control_flow::While;
use streamweave_vec::VecProducer;

#[tokio::test]
async fn test_while_transformer_basic() {
  let mut transformer = While::new(|x: &i32| *x < 5);
  let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 2, 3, 5, 4, 6]));

  let output_stream = transformer.transform(input_stream).await;
  let results: Vec<i32> = output_stream.collect().await;

  // While transformer emits items when condition becomes false (x >= 5)
  assert_eq!(results, vec![5, 6]);
}

#[tokio::test]
async fn test_while_transformer() {
  let mut while_loop = While::new(|x: &i32| *x < 5);
  let mut producer = VecProducer::new(vec![1, 2, 3, 5, 6, 7]);
  let stream = producer.produce();

  let mut output = while_loop.transform(Box::pin(stream)).await;
  let mut results = Vec::new();
  while let Some(item) = output.next().await {
    results.push(item);
  }

  // Only items where condition is false (>= 5) should be emitted
  assert_eq!(results, vec![5, 6, 7]);
}

proptest! {
  #[test]
  fn test_while_transformer_proptest(
    numbers in prop::collection::vec(-1000i32..1000, 1..50),
    threshold in -500i32..500
  ) {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async {
      let mut while_loop = While::new(move |x: &i32| *x < threshold);
      let mut producer = VecProducer::new(numbers.clone());
      let stream = producer.produce();

      let mut output = while_loop.transform(Box::pin(stream)).await;
      let mut results = Vec::new();
      while let Some(item) = output.next().await {
        results.push(item);
      }

      // Verify all results are >= threshold
      for &result in &results {
        assert!(result >= threshold);
      }

      // Verify all items < threshold are filtered out
      let expected_count = numbers.iter().filter(|&&n| n >= threshold).count();
      assert_eq!(results.len(), expected_count);
    });
  }
}
