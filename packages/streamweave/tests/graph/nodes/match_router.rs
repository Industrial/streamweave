use futures::{StreamExt, stream};
use proptest::prelude::*;
use std::pin::Pin;
use std::time::Duration;
use streamweave::Producer;
use streamweave::graph::nodes::{Match, Pattern, PredicatePattern, RangePattern};
use streamweave::graph::router::OutputRouter;
use streamweave_vec::VecProducer;

// Helper pattern for testing
struct EvenPattern;
struct OddPattern;

impl Pattern<i32> for EvenPattern {
  fn matches(&self, item: &i32) -> Option<usize> {
    if *item % 2 == 0 { Some(0) } else { None }
  }
}

impl Pattern<i32> for OddPattern {
  fn matches(&self, item: &i32) -> Option<usize> {
    if *item % 2 != 0 { Some(1) } else { None }
  }
}

#[tokio::test]
async fn test_match_router_basic() {
  let patterns: Vec<Box<dyn Pattern<i32>>> = vec![Box::new(EvenPattern), Box::new(OddPattern)];
  let mut router = Match::new(patterns, None);
  let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

  let mut output_streams = router.route_stream(input_stream).await;
  assert_eq!(output_streams.len(), 2);

  // Collect from port 0 (even)
  let mut port0_results = Vec::new();
  let stream0 = &mut output_streams[0].1;
  while let Some(item) = stream0.next().await {
    port0_results.push(item);
  }

  // Collect from port 1 (odd)
  let mut port1_results = Vec::new();
  let stream1 = &mut output_streams[1].1;
  while let Some(item) = stream1.next().await {
    port1_results.push(item);
  }

  assert_eq!(port0_results, vec![2, 4]);
  assert_eq!(port1_results, vec![1, 3, 5]);
}

#[tokio::test]
async fn test_match_router_with_default() {
  let patterns: Vec<Box<dyn Pattern<i32>>> = vec![
    Box::new(RangePattern::new(0..10, 0)),
    Box::new(RangePattern::new(10..20, 1)),
  ];
  let mut router = Match::new(patterns, Some(2));
  let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![5, 15, 25]));

  let mut output_streams = router.route_stream(input_stream).await;
  assert_eq!(output_streams.len(), 3);

  // Collect from all ports
  let mut port0_results = Vec::new();
  let stream0 = &mut output_streams[0].1;
  while let Some(item) = stream0.next().await {
    port0_results.push(item);
  }

  let mut port1_results = Vec::new();
  let stream1 = &mut output_streams[1].1;
  while let Some(item) = stream1.next().await {
    port1_results.push(item);
  }

  let mut port2_results = Vec::new();
  let stream2 = &mut output_streams[2].1;
  while let Some(item) = stream2.next().await {
    port2_results.push(item);
  }

  assert_eq!(port0_results, vec![5]);
  assert_eq!(port1_results, vec![15]);
  assert_eq!(port2_results, vec![25]); // Default port
}

#[test]
fn test_match_router_output_ports() {
  let patterns: Vec<Box<dyn Pattern<i32>>> = vec![Box::new(EvenPattern), Box::new(OddPattern)];
  let router = Match::new(patterns, None);
  let port_names = router.output_port_names();
  assert_eq!(port_names.len(), 2);
  assert!(port_names.contains(&"out".to_string()));
  assert!(port_names.contains(&"out_1".to_string()));

  let patterns: Vec<Box<dyn Pattern<i32>>> = vec![Box::new(EvenPattern)];
  let router = Match::new(patterns, Some(1));
  let port_names = router.output_port_names();
  assert_eq!(port_names.len(), 2);
  assert!(port_names.contains(&"out".to_string()));
  assert!(port_names.contains(&"out_1".to_string()));
}

#[test]
fn test_range_pattern_matches() {
  let pattern = RangePattern::new(0..10, 0);
  assert_eq!(pattern.matches(&5), Some(0));
  assert_eq!(pattern.matches(&0), Some(0));
  assert_eq!(pattern.matches(&9), Some(0));
  assert_eq!(pattern.matches(&10), None);
  assert_eq!(pattern.matches(&-1), None);
}

#[test]
fn test_range_pattern_output_ports() {
  let pattern = RangePattern::new(0..10, 5);
  assert_eq!(pattern.matches(&5), Some(5));
}

#[test]
fn test_predicate_pattern_matches() {
  let pattern = PredicatePattern::new(|x: &i32| *x > 10, 0);
  assert_eq!(pattern.matches(&15), Some(0));
  assert_eq!(pattern.matches(&5), None);
  assert_eq!(pattern.matches(&10), None);
}

#[test]
fn test_range_pattern() {
  let pattern = RangePattern::new(0..100, 0);
  assert_eq!(pattern.matches(&50), Some(0));
  assert_eq!(pattern.matches(&0), Some(0));
  assert_eq!(pattern.matches(&99), Some(0));
  assert_eq!(pattern.matches(&100), None); // End is exclusive
  assert_eq!(pattern.matches(&-1), None);
  assert_eq!(pattern.matches(&150), None);
}

#[test]
fn test_predicate_pattern() {
  let pattern = PredicatePattern::new(|x: &i32| *x % 2 == 0, 0);
  assert_eq!(pattern.matches(&2), Some(0));
  assert_eq!(pattern.matches(&4), Some(0));
  assert_eq!(pattern.matches(&1), None);
  assert_eq!(pattern.matches(&3), None);
}

#[tokio::test]
async fn test_match_router_with_range_patterns() {
  let patterns: Vec<Box<dyn Pattern<i32>>> = vec![
    Box::new(RangePattern::new(0..50, 0)),
    Box::new(RangePattern::new(50..100, 1)),
  ];
  let mut router = Match::new(patterns, Some(2)); // default to port 2

  let mut producer = VecProducer::new(vec![10, 60, 150, 30, 80, 200]);
  let stream = producer.produce();

  let mut routed = router.route_stream(Box::pin(stream)).await;
  assert_eq!(routed.len(), 3); // 2 pattern ports + 1 default port

  // Collect from all ports
  let mut port0 = Vec::new();
  let mut port1 = Vec::new();
  let mut port2 = Vec::new();

  {
    let stream0 = &mut routed[0].1;
    while let Some(item) = stream0.next().await {
      port0.push(item);
    }
  }
  {
    let stream1 = &mut routed[1].1;
    while let Some(item) = stream1.next().await {
      port1.push(item);
    }
  }
  {
    let stream2 = &mut routed[2].1;
    while let Some(item) = stream2.next().await {
      port2.push(item);
    }
  }

  // Verify routing
  assert!(port0.contains(&10) || port0.contains(&30));
  assert!(port1.contains(&60) || port1.contains(&80));
  assert!(port2.contains(&150) || port2.contains(&200));
}

#[tokio::test]
async fn test_match_router_with_predicate_patterns() {
  let patterns: Vec<Box<dyn Pattern<i32>>> = vec![
    Box::new(PredicatePattern::new(|x: &i32| *x > 100, 0)),
    Box::new(PredicatePattern::new(|x: &i32| *x < 0, 1)),
  ];
  let mut router = Match::new(patterns, None); // No default, drop unmatched

  let mut producer = VecProducer::new(vec![150, -10, 50, 200, -5]);
  let stream = producer.produce();

  let mut routed = router.route_stream(Box::pin(stream)).await;
  assert_eq!(routed.len(), 2); // 2 pattern ports, no default

  // Collect results - can't use tokio::select! with references, collect sequentially
  let mut port0 = Vec::new();
  {
    let stream0 = &mut routed[0].1;
    while let Some(item) = stream0.next().await {
      port0.push(item);
    }
  }
  let mut port1 = Vec::new();
  {
    let stream1 = &mut routed[1].1;
    while let Some(item) = stream1.next().await {
      port1.push(item);
    }
  }

  tokio::time::sleep(Duration::from_millis(100)).await;

  // Verify routing
  assert!(port0.contains(&150) || port0.contains(&200));
  assert!(port1.contains(&-10) || port1.contains(&-5));
}

proptest! {
  #[test]
  fn test_range_pattern_proptest(
    start in -1000i32..1000,
    end in -1000i32..1000,
    value in -2000i32..2000
  ) {
    if start < end {
      let pattern = RangePattern::new(start..end, 0);
      let result = pattern.matches(&value);
      if value >= start && value < end {
        assert_eq!(result, Some(0));
      } else {
        assert_eq!(result, None);
      }
    }
  }
}

proptest! {
  #![proptest_config(ProptestConfig::with_cases(5))]

  #[test]
  fn test_match_router_proptest(numbers in prop::collection::vec(-1000i32..1000, 0..20)) {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async {
      let patterns: Vec<Box<dyn Pattern<i32>>> = vec![
        Box::new(RangePattern::new(-1000..0, 0)),
        Box::new(RangePattern::new(0..500, 1)),
        Box::new(RangePattern::new(500..1000, 2)),
      ];
      let mut router = Match::new(patterns, Some(3)); // default port 3

      // Get port names before route_stream (which moves patterns)
      let port_names: Vec<String> = router.output_port_names();
      assert_eq!(port_names.len(), 4); // 3 patterns + 1 default

      let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
        Box::pin(stream::iter(numbers.clone()));

      let mut output_streams = router.route_stream(input_stream).await;
      assert_eq!(output_streams.len(), 4); // 3 patterns + 1 default

      // Collect from all ports
      let mut all_results: Vec<(String, i32)> = Vec::new();
      for (port_name, stream) in &mut output_streams {
        let s = stream;
        while let Some(item) = s.next().await {
          all_results.push((port_name.clone(), item));
        }
      }

      // Verify all items were routed
      assert_eq!(all_results.len(), numbers.len());

      // Verify routing correctness - check port names match expected pattern
      // Port names are: "out" (index 0), "out_1" (index 1), "out_2" (index 2), "out_3" (index 3)
      for (port_name, value) in all_results {
        if value < 0 {
          assert_eq!(port_name, "out"); // First pattern port
        } else if value < 500 {
          assert_eq!(port_name, "out_1"); // Second pattern port
        } else if value < 1000 {
          assert_eq!(port_name, "out_2"); // Third pattern port
        } else {
          assert_eq!(port_name, "out_3"); // Default port
        }
      }
    });
  }
}
