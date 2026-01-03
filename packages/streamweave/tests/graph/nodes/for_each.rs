use super::common::i32_vec_strategy;
use futures::{StreamExt, stream};
use proptest::prelude::*;
use std::pin::Pin;
use streamweave::graph::nodes::ForEach;
use streamweave::{Producer, Transformer};
use streamweave_vec::VecProducer;

#[tokio::test]
async fn test_for_each_transformer_basic() {
  // ForEach<T> where T = Vec<i32>, extract_collection should return Vec<Vec<i32>>
  // So we wrap each Vec<i32> in another Vec to create Vec<Vec<i32>>
  let mut transformer = ForEach::new(|vec: &Vec<i32>| vec![vec.clone()]);
  let input_stream: Pin<Box<dyn futures::Stream<Item = Vec<i32>> + Send>> =
    Box::pin(stream::iter(vec![vec![1, 2], vec![3, 4, 5]]));

  let output_stream = transformer.transform(input_stream).await;
  // ForEach outputs Vec<i32> (the collection items from Vec<Vec<i32>>)
  let results: Vec<Vec<i32>> = output_stream.collect().await;
  let flattened: Vec<i32> = results.into_iter().flatten().collect();

  assert_eq!(flattened, vec![1, 2, 3, 4, 5]);
}

#[tokio::test]
async fn test_for_each_transformer_empty_collections() {
  let mut transformer = ForEach::new(|vec: &Vec<i32>| vec![vec.clone()]);
  let input_stream: Pin<Box<dyn futures::Stream<Item = Vec<i32>> + Send>> =
    Box::pin(stream::iter(vec![vec![], vec![1], vec![]]));

  let output_stream = transformer.transform(input_stream).await;
  let results: Vec<Vec<i32>> = output_stream.collect().await;
  let flattened: Vec<i32> = results.into_iter().flatten().collect();

  assert_eq!(flattened, vec![1]);
}

#[tokio::test]
async fn test_for_each_transformer() {
  let mut for_each = ForEach::new(|item: &Vec<i32>| vec![item.clone()]);
  let mut producer = VecProducer::new(vec![vec![1, 2], vec![3, 4, 5], vec![6]]);
  let stream = producer.produce();

  let mut output = for_each.transform(Box::pin(stream)).await;
  // ForEach outputs Vec<i32> (the collection items)
  let mut results: Vec<Vec<i32>> = Vec::new();
  while let Some(item) = output.next().await {
    results.push(item);
  }
  let flattened: Vec<i32> = results.into_iter().flatten().collect();

  assert_eq!(flattened, vec![1, 2, 3, 4, 5, 6]);
}

proptest! {
  #![proptest_config(ProptestConfig::with_cases(100))]

  #[test]
  fn test_for_each_transformer_properties(collections in prop::collection::vec(i32_vec_strategy(), 0..=20)) {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async {
      let mut transformer = ForEach::new(|vec: &Vec<i32>| vec![vec.clone()]);
      let input_stream: Pin<Box<dyn futures::Stream<Item = Vec<i32>> + Send>> =
        Box::pin(stream::iter(collections.clone()));

      let output_stream = transformer.transform(input_stream).await;
      let results: Vec<Vec<i32>> = output_stream.collect().await;
      let flattened: Vec<i32> = results.into_iter().flatten().collect();

      // Verify total count matches
      let expected_count: usize = collections.iter().map(|v| v.len()).sum();
      assert_eq!(flattened.len(), expected_count);

      // Verify all items are present (order may differ, so we'll check counts)
      let mut expected_items: Vec<i32> = collections.into_iter().flatten().collect();
      expected_items.sort();
      let mut actual_items = flattened;
      actual_items.sort();
      assert_eq!(actual_items, expected_items);
    });
  }

  #[test]
  fn test_for_each_proptest(
    vectors in prop::collection::vec(
      prop::collection::vec(-1000i32..1000, 0..10),
      1..20
    )
  ) {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async {
      let mut for_each = ForEach::new(|item: &Vec<i32>| vec![item.clone()]);
      let mut producer = VecProducer::new(vectors.clone());
      let stream = producer.produce();

      let mut output = for_each.transform(Box::pin(stream)).await;
      // ForEach outputs Vec<i32> (the collection items)
      let mut results: Vec<Vec<i32>> = Vec::new();
      while let Some(item) = output.next().await {
        results.push(item);
      }
      let flattened: Vec<i32> = results.into_iter().flatten().collect();

      // Verify all items are expanded
      let expected_count: usize = vectors.iter().map(|v| v.len()).sum();
      assert_eq!(flattened.len(), expected_count);

      // Verify order is preserved
      let mut idx = 0;
      for vec in &vectors {
        for val in vec.iter() {
          assert_eq!(flattened[idx], *val);
          idx += 1;
        }
      }
    });
  }
}
