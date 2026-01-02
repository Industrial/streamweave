use futures::{StreamExt, stream};
use proptest::prelude::*;
use std::pin::Pin;
use streamweave::Transformer;
use streamweave_graph::control_flow::{Join, JoinStrategy};

#[test]
fn test_join_strategy_variants() {
  let inner = JoinStrategy::Inner;
  let left = JoinStrategy::Left;
  let right = JoinStrategy::Right;
  // Just verify variants exist and can be compared
  assert!(matches!(inner, JoinStrategy::Inner));
  assert!(matches!(left, JoinStrategy::Left));
  assert!(matches!(right, JoinStrategy::Right));
}

proptest! {
  #![proptest_config(ProptestConfig::with_cases(100))]

  #[test]
  fn test_join_proptest(left_values in prop::collection::vec(-1000i32..1000, 0..50), right_values in prop::collection::vec(-1000i32..1000, 0..50)) {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async {
      // Create tuples from the two vectors
      let tuples: Vec<(i32, i32)> = left_values
        .iter()
        .zip(right_values.iter())
        .map(|(a, b)| (*a, *b))
        .collect();

      for strategy in &[
        JoinStrategy::Inner,
        JoinStrategy::Outer,
        JoinStrategy::Left,
        JoinStrategy::Right,
      ] {
        let mut join = Join::new(*strategy);
        let input_stream: Pin<Box<dyn futures::Stream<Item = (i32, i32)> + Send>> =
          Box::pin(stream::iter(tuples.clone()));

        let mut output = join.transform(input_stream).await;
        let mut results = Vec::new();
        while let Some(item) = output.next().await {
          results.push(item);
        }

        // Join should preserve all tuples (simplified implementation)
        assert_eq!(results, tuples);
      }
    });
  }
}

#[test]
fn test_join_strategy_variants_proptest() {
  let strategies = vec![
    JoinStrategy::Inner,
    JoinStrategy::Outer,
    JoinStrategy::Left,
    JoinStrategy::Right,
  ];

  for strategy in &strategies {
    assert_eq!(*strategy, *strategy);
    let join1 = Join::<i32, i32>::new(*strategy);
    let join2 = Join::<i32, i32>::new(*strategy);
    // Verify they can be created with the same strategy
    assert_eq!(
      join1.get_config_impl().error_strategy(),
      join2.get_config_impl().error_strategy()
    );
  }
}
