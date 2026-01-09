//! Tests for distributed partitioner module

use streamweave::distributed::partitioner::{
  HashPartitioner, PartitionKey, PartitionStrategy, Partitioner, RoundRobinPartitioner,
};

#[test]
fn test_partition_key_from_string() {
  let key: PartitionKey = "test".to_string().into();
  match key {
    PartitionKey::String(s) => assert_eq!(s, "test"),
    _ => assert!(false),
  }
}

#[test]
fn test_partition_key_from_u64() {
  let key: PartitionKey = 42u64.into();
  match key {
    PartitionKey::Numeric(n) => assert_eq!(n, 42),
    _ => assert!(false),
  }
}

#[test]
fn test_partition_key_from_bytes() {
  let key: PartitionKey = vec![1, 2, 3].into();
  match key {
    PartitionKey::Bytes(b) => assert_eq!(b, vec![1, 2, 3]),
    _ => assert!(false),
  }
}

#[test]
fn test_partition_strategy_default() {
  let strategy = PartitionStrategy::default();
  assert_eq!(strategy, PartitionStrategy::Hash);
}

#[test]
fn test_partition_strategy_variants() {
  assert_eq!(PartitionStrategy::Hash, PartitionStrategy::Hash);
  assert_eq!(PartitionStrategy::Range, PartitionStrategy::Range);
  assert_eq!(PartitionStrategy::RoundRobin, PartitionStrategy::RoundRobin);
  assert_eq!(PartitionStrategy::Broadcast, PartitionStrategy::Broadcast);
  assert_eq!(PartitionStrategy::Custom, PartitionStrategy::Custom);
}

#[test]
fn test_hash_partitioner_new() {
  let partitioner = HashPartitioner::new();
  assert_eq!(partitioner.strategy(), PartitionStrategy::Hash);
}

#[test]
fn test_hash_partitioner_default() {
  let partitioner = HashPartitioner::default();
  assert_eq!(partitioner.strategy(), PartitionStrategy::Hash);
}

#[test]
fn test_hash_partitioner_partition() {
  let partitioner = HashPartitioner::new();
  let key = PartitionKey::String("test".to_string());

  // Should return a valid partition index
  let partition = partitioner.partition(&key, 5);
  assert!(partition < 5);
}

#[test]
fn test_hash_partitioner_partition_zero_partitions() {
  let partitioner = HashPartitioner::new();
  let key = PartitionKey::String("test".to_string());

  // Should return 0 when num_partitions is 0
  let partition = partitioner.partition(&key, 0);
  assert_eq!(partition, 0);
}

#[test]
fn test_hash_partitioner_consistent_hashing() {
  let partitioner = HashPartitioner::new();
  let key = PartitionKey::String("test".to_string());

  // Same key should always map to same partition
  let partition1 = partitioner.partition(&key, 5);
  let partition2 = partitioner.partition(&key, 5);
  assert_eq!(partition1, partition2);
}

#[test]
fn test_round_robin_partitioner_new() {
  let partitioner = RoundRobinPartitioner::new();
  assert_eq!(partitioner.strategy(), PartitionStrategy::RoundRobin);
}

#[test]
fn test_round_robin_partitioner_default() {
  let partitioner = RoundRobinPartitioner::default();
  assert_eq!(partitioner.strategy(), PartitionStrategy::RoundRobin);
}

#[test]
fn test_round_robin_partitioner_partition() {
  let partitioner = RoundRobinPartitioner::new();
  let key = PartitionKey::String("test".to_string());

  // Should cycle through partitions
  let partition1 = partitioner.partition(&key, 3);
  let partition2 = partitioner.partition(&key, 3);
  let partition3 = partitioner.partition(&key, 3);
  let partition4 = partitioner.partition(&key, 3);

  // Should cycle: 0, 1, 2, 0, ...
  assert_eq!(partition1, 0);
  assert_eq!(partition2, 1);
  assert_eq!(partition3, 2);
  assert_eq!(partition4, 0);
}

#[test]
fn test_round_robin_partitioner_partition_zero_partitions() {
  let partitioner = RoundRobinPartitioner::new();
  let key = PartitionKey::String("test".to_string());

  // Should return 0 when num_partitions is 0
  let partition = partitioner.partition(&key, 0);
  assert_eq!(partition, 0);
}
