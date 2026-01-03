use super::common::i32_strategy;
use futures::StreamExt;
use proptest::prelude::*;
use streamweave::graph::nodes::{
  Aggregate, Aggregator, CountAggregator, MaxAggregator, MinAggregator, SumAggregator,
};
use streamweave::{Producer, Transformer};
use streamweave_vec::VecProducer;

#[test]
fn test_sum_aggregator() {
  let aggregator = SumAggregator;
  let mut acc = aggregator.init();
  aggregator.accumulate(&mut acc, &1);
  aggregator.accumulate(&mut acc, &2);
  aggregator.accumulate(&mut acc, &3);
  let result = aggregator.finalize(acc);
  assert_eq!(result, 6);
}

#[test]
fn test_count_aggregator() {
  let aggregator = CountAggregator;
  let mut acc = <CountAggregator as Aggregator<i32, usize>>::init(&aggregator);
  <CountAggregator as Aggregator<i32, usize>>::accumulate(&aggregator, &mut acc, &1);
  <CountAggregator as Aggregator<i32, usize>>::accumulate(&aggregator, &mut acc, &2);
  <CountAggregator as Aggregator<i32, usize>>::accumulate(&aggregator, &mut acc, &3);
  let result = <CountAggregator as Aggregator<i32, usize>>::finalize(&aggregator, acc);
  assert_eq!(result, 3);
}

#[test]
fn test_min_aggregator() {
  let aggregator = MinAggregator;
  let mut acc = aggregator.init();
  aggregator.accumulate(&mut acc, &5);
  aggregator.accumulate(&mut acc, &2);
  aggregator.accumulate(&mut acc, &8);
  let result = aggregator.finalize(acc);
  assert_eq!(result, Some(2));
}

#[test]
fn test_min_aggregator_empty() {
  let aggregator = MinAggregator;
  let acc: Option<i32> = aggregator.init();
  let result = aggregator.finalize(acc);
  assert_eq!(result, None);
}

#[test]
fn test_max_aggregator() {
  let aggregator = MaxAggregator;
  let mut acc = aggregator.init();
  aggregator.accumulate(&mut acc, &5);
  aggregator.accumulate(&mut acc, &2);
  aggregator.accumulate(&mut acc, &8);
  let result = aggregator.finalize(acc);
  assert_eq!(result, Some(8));
}

#[test]
fn test_max_aggregator_empty() {
  let aggregator = MaxAggregator;
  let acc: Option<i32> = aggregator.init();
  let result = aggregator.finalize(acc);
  assert_eq!(result, None);
}

#[tokio::test]
async fn test_aggregate_transformer_with_sum() {
  let mut aggregate = Aggregate::new(SumAggregator, None);
  let mut producer = VecProducer::new(vec![1, 2, 3, 4, 5]);
  let stream = producer.produce();

  let mut output = aggregate.transform(Box::pin(stream)).await;
  let mut results = Vec::new();
  while let Some(item) = output.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![15]); // Sum of all items
}

#[tokio::test]
async fn test_aggregate_transformer_with_window() {
  let mut aggregate = Aggregate::new(SumAggregator, Some(3));
  let mut producer = VecProducer::new(vec![1, 2, 3, 4, 5, 6]);
  let stream = producer.produce();

  let mut output = aggregate.transform(Box::pin(stream)).await;
  let mut results = Vec::new();
  while let Some(item) = output.next().await {
    results.push(item);
  }

  // Should emit sum of first 3, then sum of next 3
  assert_eq!(results, vec![6, 15]);
}

#[tokio::test]
async fn test_aggregate_transformer_with_count() {
  let mut aggregate = Aggregate::new(CountAggregator, None);
  let mut producer = VecProducer::new(vec![1, 2, 3, 4, 5]);
  let stream = producer.produce();

  let mut output = aggregate.transform(Box::pin(stream)).await;
  let mut results = Vec::new();
  while let Some(item) = output.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![5]); // Count of all items
}

#[tokio::test]
async fn test_aggregate_transformer_with_min() {
  let mut aggregate = Aggregate::new(MinAggregator, None);
  let mut producer = VecProducer::new(vec![5, 2, 8, 1, 9]);
  let stream = producer.produce();

  let mut output = aggregate.transform(Box::pin(stream)).await;
  let mut results = Vec::new();
  while let Some(item) = output.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![Some(1)]);
}

#[tokio::test]
async fn test_aggregate_transformer_with_max() {
  let mut aggregate = Aggregate::new(MaxAggregator, None);
  let mut producer = VecProducer::new(vec![5, 2, 8, 1, 9]);
  let stream = producer.produce();

  let mut output = aggregate.transform(Box::pin(stream)).await;
  let mut results = Vec::new();
  while let Some(item) = output.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![Some(9)]);
}

proptest! {
  #[test]
  fn test_sum_aggregator_properties(items in prop::collection::vec(i32_strategy(), 1..=100)) {
    let aggregator = SumAggregator;
    let mut acc = aggregator.init();
    for item in &items {
      aggregator.accumulate(&mut acc, item);
    }
    let result = aggregator.finalize(acc);
    let expected: i32 = items.iter().sum();
    prop_assert_eq!(result, expected);
  }

  #[test]
  fn test_count_aggregator_properties(items in prop::collection::vec(any::<i32>(), 0..=100)) {
    let aggregator = CountAggregator;
    let mut acc = <CountAggregator as Aggregator<i32, usize>>::init(&aggregator);
    for item in &items {
      <CountAggregator as Aggregator<i32, usize>>::accumulate(&aggregator, &mut acc, item);
    }
    let result = <CountAggregator as Aggregator<i32, usize>>::finalize(&aggregator, acc);
    prop_assert_eq!(result, items.len());
  }

  #[test]
  fn test_min_aggregator_properties(items in prop::collection::vec(i32_strategy(), 1..=100)) {
    let aggregator = MinAggregator;
    let mut acc = aggregator.init();
    for item in &items {
      aggregator.accumulate(&mut acc, item);
    }
    let result = aggregator.finalize(acc);
    let expected = items.iter().min().copied();
    prop_assert_eq!(result, expected);
  }

  #[test]
  fn test_max_aggregator_properties(items in prop::collection::vec(i32_strategy(), 1..=100)) {
    let aggregator = MaxAggregator;
    let mut acc = aggregator.init();
    for item in &items {
      aggregator.accumulate(&mut acc, item);
    }
    let result = aggregator.finalize(acc);
    let expected = items.iter().max().copied();
    prop_assert_eq!(result, expected);
  }

  #[test]
  fn test_aggregate_sum_proptest(
    numbers in prop::collection::vec(-1000i32..1000, 1..100)
  ) {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async {
      let mut aggregate = Aggregate::new(SumAggregator, None);
      let mut producer = VecProducer::new(numbers.clone());
      let stream = producer.produce();

      let mut output = aggregate.transform(Box::pin(stream)).await;
      let mut results = Vec::new();
      while let Some(item) = output.next().await {
        results.push(item);
      }

      let expected_sum: i32 = numbers.iter().sum();
      assert_eq!(results, vec![expected_sum]);
    });
  }

  #[test]
  fn test_sum_aggregator_proptest(
    numbers in prop::collection::vec(-1000i32..1000, 1..100)
  ) {
    let agg = SumAggregator;
    let mut acc = agg.init();
    let expected_sum: i32 = numbers.iter().sum();

    for num in &numbers {
      agg.accumulate(&mut acc, num);
    }

    let result = agg.finalize(acc);
    assert_eq!(result, expected_sum);
  }

  #[test]
  fn test_min_aggregator_proptest(
    numbers in prop::collection::vec(-1000i32..1000, 1..100)
  ) {
    if !numbers.is_empty() {
      let agg = MinAggregator;
      let mut acc = agg.init();
      let expected_min = *numbers.iter().min().unwrap();

      for num in &numbers {
        agg.accumulate(&mut acc, num);
      }

      let result = agg.finalize(acc);
      assert_eq!(result, Some(expected_min));
    }
  }

  #[test]
  fn test_max_aggregator_proptest(
    numbers in prop::collection::vec(-1000i32..1000, 1..100)
  ) {
    if !numbers.is_empty() {
      let agg = MaxAggregator;
      let mut acc = agg.init();
      let expected_max = *numbers.iter().max().unwrap();

      for num in &numbers {
        agg.accumulate(&mut acc, num);
      }

      let result = agg.finalize(acc);
      assert_eq!(result, Some(expected_max));
    }
  }
}
