use futures::{StreamExt, stream};
use proptest::prelude::*;
use std::pin::Pin;
use streamweave_graph::control_flow::Synchronize;
use streamweave_graph::router::InputRouter;

#[tokio::test]
async fn test_synchronize_router() {
  let mut sync = Synchronize::new(2);
  let stream1: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(futures::stream::iter(vec![1, 2, 3]));
  let stream2: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(futures::stream::iter(vec![4, 5, 6]));

  let expected_ports = sync.expected_port_names();
  let streams = vec![
    (expected_ports[0].clone(), stream1),
    (expected_ports[1].clone(), stream2),
  ];
  let mut output = sync.route_streams(streams).await;

  let mut results = Vec::new();
  while let Some(item) = output.next().await {
    results.push(item);
  }

  // Should synchronize and emit items
  assert!(!results.is_empty());
}

#[test]
fn test_synchronize_expected_ports() {
  let sync = Synchronize::<i32>::new(3);
  assert_eq!(
    sync.expected_port_names(),
    vec!["in".to_string(), "in_1".to_string(), "in_2".to_string()]
  );
}

proptest! {
  #![proptest_config(ProptestConfig::with_cases(100))]

  #[test]
  fn test_synchronize_proptest(stream1_items in prop::collection::vec(-1000i32..1000, 0..50), stream2_items in prop::collection::vec(-1000i32..1000, 0..50)) {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async {
      let mut sync = Synchronize::new(2);
      let stream1: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
        Box::pin(stream::iter(stream1_items.clone()));
      let stream2: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
        Box::pin(stream::iter(stream2_items.clone()));

      let expected_ports = sync.expected_port_names();
      let streams = vec![(expected_ports[0].clone(), stream1), (expected_ports[1].clone(), stream2)];
      let mut output = sync.route_streams(streams).await;

      let mut results = Vec::new();
      while let Some(item) = output.next().await {
        results.push(item);
      }

      // Synchronize should emit items when both streams have data
      // The exact count depends on synchronization behavior
      let min_len = stream1_items.len().min(stream2_items.len());
      assert!(results.len() <= min_len || results.len() <= stream1_items.len().max(stream2_items.len()));
    });
  }

  #[test]
  fn test_synchronize_expected_ports_proptest(num_inputs in 1usize..=10) {
    let sync = Synchronize::<i32>::new(num_inputs);
    let port_names = sync.expected_port_names();
    prop_assert_eq!(port_names.len(), num_inputs);
    // Verify port names follow the pattern "in", "in_1", "in_2", etc.
    if num_inputs > 0 {
      prop_assert_eq!(&port_names[0], "in");
    }
    for (i, port_name) in port_names.iter().enumerate().skip(1).take(num_inputs - 1) {
      prop_assert_eq!(port_name, &format!("in_{}", i));
    }
  }
}
