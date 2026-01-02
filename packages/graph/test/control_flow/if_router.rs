use super::common::i32_strategy;
use futures::{StreamExt, stream};
use proptest::prelude::*;
use std::pin::Pin;
use streamweave::Producer;
use streamweave_graph::control_flow::If;
use streamweave_graph::router::OutputRouter;

#[tokio::test]
async fn test_if_router_basic() {
  let mut router = If::new(|x: &i32| *x % 2 == 0);
  let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

  let mut output_streams = router.route_stream(input_stream).await;
  assert_eq!(output_streams.len(), 2);

  // Collect from true port (even numbers)
  let mut true_results = Vec::new();
  let stream_true = &mut output_streams[0].1;
  while let Some(item) = stream_true.next().await {
    true_results.push(item);
  }

  // Collect from false port (odd numbers)
  let mut false_results = Vec::new();
  let stream_false = &mut output_streams[1].1;
  while let Some(item) = stream_false.next().await {
    false_results.push(item);
  }

  assert_eq!(true_results, vec![2, 4]);
  assert_eq!(false_results, vec![1, 3, 5]);
}

#[tokio::test]
async fn test_if_router_all_true() {
  let mut router = If::new(|x: &i32| *x > 0);
  let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_streams = router.route_stream(input_stream).await;

  let mut true_results = Vec::new();
  let stream_true = &mut output_streams[0].1;
  while let Some(item) = stream_true.next().await {
    true_results.push(item);
  }

  assert_eq!(true_results, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_if_router_all_false() {
  let mut router = If::new(|x: &i32| *x < 0);
  let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_streams = router.route_stream(input_stream).await;

  let mut false_results = Vec::new();
  let stream_false = &mut output_streams[1].1;
  while let Some(item) = stream_false.next().await {
    false_results.push(item);
  }

  assert_eq!(false_results, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_if_router_empty_stream() {
  let mut router = If::new(|x: &i32| *x % 2 == 0);
  let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![]));

  let mut output_streams = router.route_stream(input_stream).await;
  assert_eq!(output_streams.len(), 2);

  let stream_true = &mut output_streams[0].1;
  assert!(stream_true.next().await.is_none());

  let stream_false = &mut output_streams[1].1;
  assert!(stream_false.next().await.is_none());
}

#[test]
fn test_if_router_output_ports() {
  let router = If::<i32>::new(|_| true);
  assert_eq!(router.output_ports(), vec![0, 1]);
}

proptest! {
  #![proptest_config(ProptestConfig::with_cases(5))]

  #[test]
  fn test_if_router_properties(items in prop::collection::vec(i32_strategy(), 0..=20), threshold in i32_strategy()) {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async {
      let mut router = If::new(move |x: &i32| *x >= threshold);
      let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
        Box::pin(stream::iter(items.clone()));

      let mut output_streams = router.route_stream(input_stream).await;
      assert_eq!(output_streams.len(), 2);

      let mut true_results = Vec::new();
      let stream_true = &mut output_streams[0].1;
      while let Some(item) = stream_true.next().await {
        true_results.push(item);
      }

      let mut false_results = Vec::new();
      let stream_false = &mut output_streams[1].1;
      while let Some(item) = stream_false.next().await {
        false_results.push(item);
      }

      // Verify all items are accounted for
      assert_eq!(true_results.len() + false_results.len(), items.len());

      // Verify all true results meet the predicate
      for item in &true_results {
        assert!(*item >= threshold);
      }

      // Verify all false results don't meet the predicate
      for item in &false_results {
        assert!(*item < threshold);
      }
    });
  }

  #[test]
  fn test_if_router_proptest(numbers in prop::collection::vec(-1000i32..1000, 1..20)) {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async {
      let mut router = If::new(|x: &i32| *x >= 0);
      let mut producer = streamweave_vec::VecProducer::new(numbers.clone());
      let stream = producer.produce();

      let mut routed = router.route_stream(Box::pin(stream)).await;
      assert_eq!(routed.len(), 2);

      // Collect from both ports
      let mut true_results = Vec::new();
      {
        let true_stream = &mut routed[0].1;
        while let Some(item) = true_stream.next().await {
          true_results.push(item);
        }
      }
      let mut false_results = Vec::new();
      {
        let false_stream = &mut routed[1].1;
        while let Some(item) = false_stream.next().await {
          false_results.push(item);
        }
      }

      // Verify all numbers are accounted for
      let total = true_results.len() + false_results.len();
      assert_eq!(total, numbers.len());

      // Verify partitioning
      for &num in &true_results {
        assert!(num >= 0);
      }
      for &num in &false_results {
        assert!(num < 0);
      }
    });
  }
}
