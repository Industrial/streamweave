use futures::{StreamExt, stream};
use proptest::prelude::*;
use std::pin::Pin;
use streamweave::{Producer, Transformer};
use streamweave_graph::control_flow::GroupBy;
use streamweave_vec::VecProducer;

#[tokio::test]
async fn test_group_by_transformer_basic() {
  let mut transformer = GroupBy::new(|x: &i32| *x % 3);
  let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 2, 3, 4, 5, 6, 7]));

  let output_stream = transformer.transform(input_stream).await;
  let results: Vec<(i32, Vec<i32>)> = output_stream.collect().await;

  // Should have groups for mod 3 values: 0, 1, 2
  assert_eq!(results.len(), 3);

  // Verify groups contain correct items
  let mut group_map: std::collections::HashMap<i32, Vec<i32>> = results.into_iter().collect();

  let group0 = group_map.remove(&0).unwrap();
  assert_eq!(group0, vec![3, 6]);

  let group1 = group_map.remove(&1).unwrap();
  assert_eq!(group1, vec![1, 4, 7]);

  let group2 = group_map.remove(&2).unwrap();
  assert_eq!(group2, vec![2, 5]);
}

#[tokio::test]
async fn test_group_by_transformer_empty_stream() {
  let mut transformer = GroupBy::new(|x: &i32| *x % 2);
  let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![]));

  let output_stream = transformer.transform(input_stream).await;
  let results: Vec<(i32, Vec<i32>)> = output_stream.collect().await;

  assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_group_by_transformer() {
  let mut group_by = GroupBy::new(|item: &(String, i32)| item.0.clone());
  let mut producer = VecProducer::new(vec![
    ("a".to_string(), 1),
    ("b".to_string(), 2),
    ("a".to_string(), 3),
    ("b".to_string(), 4),
  ]);
  let stream = producer.produce();

  let mut output = group_by.transform(Box::pin(stream)).await;
  let mut results = Vec::new();
  while let Some(item) = output.next().await {
    results.push(item);
  }

  // Should have 2 groups
  assert_eq!(results.len(), 2);

  // Verify grouping
  let groups: std::collections::HashMap<String, Vec<(String, i32)>> = results.into_iter().collect();
  assert_eq!(groups.get("a").unwrap().len(), 2);
  assert_eq!(groups.get("b").unwrap().len(), 2);
}

proptest! {
  #[test]
  fn test_group_by_proptest(
    items in prop::collection::vec(
      (prop::string::string_regex("[a-z]{1,3}").unwrap(), -100i32..100),
      1..50
    )
  ) {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async {
      let mut group_by = GroupBy::new(|item: &(String, i32)| item.0.clone());
      let mut producer = VecProducer::new(items.clone());
      let stream = producer.produce();

      let mut output = group_by.transform(Box::pin(stream)).await;
      let mut results = Vec::new();
      while let Some(item) = output.next().await {
        results.push(item);
      }

      // Verify all items are accounted for
      let total_items: usize = results.iter().map(|(_, v)| v.len()).sum();
      assert_eq!(total_items, items.len());

      // Verify each group contains correct items
      let groups: std::collections::HashMap<String, Vec<(String, i32)>> =
        results.into_iter().collect();
      for (key, group) in &groups {
        for item in group {
          assert_eq!(&item.0, key);
        }
      }
    });
  }
}
