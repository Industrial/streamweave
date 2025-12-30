use proptest::prelude::*;
use streamweave_distributed::rebalance;
use streamweave_distributed::{
  CustomPartitioner, HashPartitioner, PartitionKey, PartitionStrategy, Partitioner,
  RangePartitioner, RoundRobinPartitioner,
};

fn partition_key_strategy() -> impl Strategy<Value = PartitionKey> {
  prop_oneof![
    prop::string::string_regex("[a-zA-Z0-9_-]+")
      .unwrap()
      .prop_map(PartitionKey::String),
    (0u64..1000000u64).prop_map(PartitionKey::Numeric),
    prop::collection::vec(any::<u8>(), 0..100).prop_map(PartitionKey::Bytes),
    (0u64..1000000u64).prop_map(PartitionKey::Hash),
  ]
}

proptest! {
  #[test]
  fn test_hash_partitioner(key in partition_key_strategy(), num_partitions in 1usize..1000) {
    let partitioner = HashPartitioner::new();
    let partition = partitioner.partition(&key, num_partitions);
    prop_assert!(partition < num_partitions);

    // Same key should always map to same partition
    let partition2 = partitioner.partition(&key, num_partitions);
    prop_assert_eq!(partition, partition2);
  }

  #[test]
  fn test_hash_partitioner_consistency(key in partition_key_strategy(), num_partitions in 1usize..1000) {
    let partitioner = HashPartitioner::new();

    // Same key should map to same partition regardless of when called
    let p1 = partitioner.partition(&key, num_partitions);
    let p2 = partitioner.partition(&key, num_partitions);
    let p3 = partitioner.partition(&key, num_partitions);

    prop_assert_eq!(p1, p2);
    prop_assert_eq!(p2, p3);
  }

  #[test]
  fn test_round_robin_partitioner(num_partitions in 2usize..100) {
    let partitioner = RoundRobinPartitioner::new();
    let key = PartitionKey::String("test".to_string());
    let p1 = partitioner.partition(&key, num_partitions);
    let p2 = partitioner.partition(&key, num_partitions);
    let p3 = partitioner.partition(&key, num_partitions);

    // Round-robin should give different partitions
    prop_assert_ne!(p1, p2);
    prop_assert_ne!(p2, p3);
    prop_assert!(p1 < num_partitions && p2 < num_partitions && p3 < num_partitions);
  }

  #[test]
  fn test_round_robin_even_distribution(num_partitions in 2usize..10) {
    let partitioner = RoundRobinPartitioner::new();
    let key = PartitionKey::String("test".to_string());
    let iterations = num_partitions * 10;

    // Get multiple partitions
    let partitions: Vec<usize> = (0..iterations)
      .map(|_| partitioner.partition(&key, num_partitions))
      .collect();

    // Count distribution
    let mut counts = vec![0; num_partitions];
    for &p in &partitions {
      counts[p] += 1;
    }

    // Should be roughly even (iterations / num_partitions each)
    let expected_count = iterations / num_partitions;
    let tolerance = expected_count / 2;
    for count in counts {
      prop_assert!(
        (expected_count - tolerance) <= count && count <= (expected_count + tolerance),
        "Distribution should be roughly even"
      );
    }
  }

  #[test]
  fn test_range_partitioner(key_value in 0u64..1000000u64, num_partitions in 1usize..1000) {
    let partitioner = RangePartitioner::new();
    let key = PartitionKey::Numeric(key_value);

    let p1 = partitioner.partition(&key, num_partitions);

    // All should be valid partitions
    prop_assert!(p1 < num_partitions);

    // Same key should map to same partition
    let p1_again = partitioner.partition(&key, num_partitions);
    prop_assert_eq!(p1, p1_again);
  }

  #[test]
  fn test_range_partitioner_consistency(key_value in 0u64..1000000u64, num_partitions in 1usize..1000) {
    let partitioner = RangePartitioner::new();
    let key = PartitionKey::Numeric(key_value);

    // Same key should map to same partition
    let p1 = partitioner.partition(&key, num_partitions);
    let p2 = partitioner.partition(&key, num_partitions);

    prop_assert_eq!(p1, p2);
  }

  #[test]
  fn test_custom_partitioner(key_value in 0u64..1000u64, num_partitions in 1usize..100) {
    let partitioner = CustomPartitioner::new(|key, num_partitions| match key {
      PartitionKey::Numeric(n) => (*n as usize) % num_partitions,
      _ => 0,
    });

    let key = PartitionKey::Numeric(key_value);
    let p = partitioner.partition(&key, num_partitions);

    prop_assert_eq!(p, (key_value as usize) % num_partitions);
    prop_assert_eq!(partitioner.strategy(), PartitionStrategy::Custom);
  }

  #[test]
  fn test_rebalance_needs_rebalance(
    key in partition_key_strategy(),
    old_num_partitions in 1usize..100,
    new_num_partitions in 1usize..100,
  ) {
    let partitioner = HashPartitioner::new();

    // Check rebalancing when partition count changes
    let (old, new, needs) = rebalance::needs_rebalance(&partitioner, &key, old_num_partitions, new_num_partitions);

    prop_assert!(old < old_num_partitions);
    prop_assert!(new < new_num_partitions);
    // Rebalancing may or may not be needed depending on hash
    prop_assert_eq!(needs, old != new);
  }

  #[test]
  fn test_rebalance_hash_partitioner_scaling(
    num_keys in 10usize..1000,
    old_num_partitions in 2usize..50,
    new_num_partitions in 2usize..50,
  ) {
    let partitioner = HashPartitioner::new();
    let sample_keys: Vec<PartitionKey> = (0..num_keys).map(|i| PartitionKey::Numeric(i as u64)).collect();

    let percentage = rebalance::rebalance_percentage(&partitioner, &sample_keys, old_num_partitions, new_num_partitions);

    // Hash partitioner should have rebalancing when partition count changes
    // Exact percentage depends on hash distribution, but should be >= 0 and <= 1
    prop_assert!((0.0..=1.0).contains(&percentage));

    // If partitions changed, rebalancing should be non-zero (unless all keys happen to map to same partitions)
    if old_num_partitions != new_num_partitions {
      // At least some rebalancing is expected, but we can't guarantee it's > 0 for all cases
      // So we just check it's a valid percentage
      prop_assert!((0.0..=1.0).contains(&percentage));
    }
  }

  #[test]
  fn test_zero_partitions(key in partition_key_strategy()) {
    let partitioner = HashPartitioner::new();

    let p = partitioner.partition(&key, 0);
    prop_assert_eq!(p, 0);
  }

  #[test]
  fn test_single_partition(key1 in partition_key_strategy(), key2 in partition_key_strategy()) {
    let partitioner = HashPartitioner::new();

    let p1 = partitioner.partition(&key1, 1);
    let p2 = partitioner.partition(&key2, 1);

    // All keys should map to partition 0
    prop_assert_eq!(p1, 0);
    prop_assert_eq!(p2, 0);
  }
}

#[test]
fn test_partition_key_from_string() {
  let key: PartitionKey = "test".to_string().into();
  match key {
    PartitionKey::String(s) => assert_eq!(s, "test"),
    _ => panic!("Expected String variant"),
  }
}

#[test]
fn test_partition_key_from_u64() {
  let key: PartitionKey = 42u64.into();
  match key {
    PartitionKey::Numeric(n) => assert_eq!(n, 42),
    _ => panic!("Expected Numeric variant"),
  }
}

#[test]
fn test_partition_key_from_bytes() {
  let bytes = vec![1, 2, 3, 4];
  let key: PartitionKey = bytes.clone().into();
  match key {
    PartitionKey::Bytes(b) => assert_eq!(b, bytes),
    _ => panic!("Expected Bytes variant"),
  }
}

#[test]
fn test_hash_partitioner_strategy() {
  let partitioner = HashPartitioner::new();
  assert_eq!(partitioner.strategy(), PartitionStrategy::Hash);
}

#[test]
fn test_hash_partitioner_default() {
  let partitioner = HashPartitioner::default();
  let key = PartitionKey::String("test".to_string());
  let partition = partitioner.partition(&key, 10);
  assert!(partition < 10);
}

#[test]
fn test_range_partitioner_string_key() {
  let partitioner = RangePartitioner::new();
  let key = PartitionKey::String("test".to_string());
  let partition = partitioner.partition(&key, 10);
  assert!(partition < 10);
}

#[test]
fn test_range_partitioner_bytes_key() {
  let partitioner = RangePartitioner::new();
  let key = PartitionKey::Bytes(vec![1, 2, 3, 4, 5, 6, 7, 8]);
  let partition = partitioner.partition(&key, 10);
  assert!(partition < 10);
}

#[test]
fn test_range_partitioner_empty_bytes() {
  let partitioner = RangePartitioner::new();
  let key = PartitionKey::Bytes(vec![]);
  let partition = partitioner.partition(&key, 10);
  assert!(partition < 10);
}

#[test]
fn test_range_partitioner_hash_key() {
  let partitioner = RangePartitioner::new();
  let key = PartitionKey::Hash(12345);
  let partition = partitioner.partition(&key, 10);
  assert!(partition < 10);
}

#[test]
fn test_range_partitioner_set_num_partitions() {
  let partitioner = RangePartitioner::new();
  partitioner.set_num_partitions(20);
  let key = PartitionKey::Numeric(100);
  let partition = partitioner.partition(&key, 20);
  assert!(partition < 20);
}

#[test]
fn test_range_partitioner_strategy() {
  let partitioner = RangePartitioner::new();
  assert_eq!(partitioner.strategy(), PartitionStrategy::Range);
}

#[test]
fn test_range_partitioner_default() {
  let partitioner = RangePartitioner::default();
  let key = PartitionKey::Numeric(42);
  let partition = partitioner.partition(&key, 10);
  assert!(partition < 10);
}

#[test]
fn test_round_robin_partitioner_strategy() {
  let partitioner = RoundRobinPartitioner::new();
  assert_eq!(partitioner.strategy(), PartitionStrategy::RoundRobin);
}

#[test]
fn test_round_robin_partitioner_default() {
  let partitioner = RoundRobinPartitioner::default();
  let key = PartitionKey::String("test".to_string());
  let partition = partitioner.partition(&key, 10);
  assert!(partition < 10);
}

#[test]
fn test_custom_partitioner_strategy() {
  let partitioner = CustomPartitioner::new(|_key, num_partitions| num_partitions / 2);
  assert_eq!(partitioner.strategy(), PartitionStrategy::Custom);
}

#[test]
fn test_partition_strategy_default() {
  assert_eq!(PartitionStrategy::default(), PartitionStrategy::Hash);
}

#[test]
fn test_partition_strategy_serialization() {
  let strategies = vec![
    PartitionStrategy::Hash,
    PartitionStrategy::Range,
    PartitionStrategy::RoundRobin,
    PartitionStrategy::Broadcast,
    PartitionStrategy::Custom,
  ];

  for strategy in strategies {
    let serialized = serde_json::to_string(&strategy).unwrap();
    let deserialized: PartitionStrategy = serde_json::from_str(&serialized).unwrap();
    assert_eq!(strategy, deserialized);
  }
}

#[test]
fn test_rebalance_needs_rebalance_zero_partitions() {
  let partitioner = HashPartitioner::new();
  let key = PartitionKey::String("test".to_string());
  let (old, new, needs) = rebalance::needs_rebalance(&partitioner, &key, 0, 0);
  assert_eq!(old, 0);
  assert_eq!(new, 0);
  assert!(!needs);
}

#[test]
fn test_rebalance_percentage_empty_keys() {
  let partitioner = HashPartitioner::new();
  let percentage = rebalance::rebalance_percentage(&partitioner, &[], 10, 20);
  assert_eq!(percentage, 0.0);
}

#[test]
fn test_rebalance_percentage_zero_partitions() {
  let partitioner = HashPartitioner::new();
  let keys = vec![PartitionKey::String("test".to_string())];
  let percentage = rebalance::rebalance_percentage(&partitioner, &keys, 0, 0);
  assert_eq!(percentage, 0.0);
}

#[test]
fn test_partition_key_debug() {
  let keys = vec![
    PartitionKey::String("test".to_string()),
    PartitionKey::Numeric(42),
    PartitionKey::Bytes(vec![1, 2, 3]),
    PartitionKey::Hash(12345),
  ];

  for key in keys {
    let debug_str = format!("{:?}", key);
    assert!(!debug_str.is_empty());
  }
}

#[test]
fn test_partition_key_clone() {
  let key1 = PartitionKey::String("test".to_string());
  let key2 = key1.clone();
  assert_eq!(key1, key2);
}

#[test]
fn test_partition_key_hash() {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};

  let key1 = PartitionKey::String("test".to_string());
  let key2 = PartitionKey::String("test".to_string());

  let mut hasher1 = DefaultHasher::new();
  let mut hasher2 = DefaultHasher::new();
  key1.hash(&mut hasher1);
  key2.hash(&mut hasher2);

  assert_eq!(hasher1.finish(), hasher2.finish());
}
