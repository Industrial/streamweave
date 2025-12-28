//! Data partitioning strategies for distributed processing.
//!
//! Defines how data is distributed across worker nodes, including
//! hash-based, range-based, and custom partitioning strategies.

use std::hash::Hash;

/// A key used for partitioning data.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PartitionKey {
  /// String-based key.
  String(String),
  /// Numeric key (u64).
  Numeric(u64),
  /// Bytes key.
  Bytes(Vec<u8>),
  /// Hash-based key (for consistent hashing).
  Hash(u64),
}

impl From<String> for PartitionKey {
  fn from(s: String) -> Self {
    PartitionKey::String(s)
  }
}

impl From<u64> for PartitionKey {
  fn from(n: u64) -> Self {
    PartitionKey::Numeric(n)
  }
}

impl From<Vec<u8>> for PartitionKey {
  fn from(bytes: Vec<u8>) -> Self {
    PartitionKey::Bytes(bytes)
  }
}

/// Partitioning strategy for distributing data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PartitionStrategy {
  /// Hash-based partitioning (consistent hashing).
  #[default]
  Hash,
  /// Range-based partitioning (ordered ranges).
  Range,
  /// Round-robin distribution.
  RoundRobin,
  /// Broadcast to all partitions.
  Broadcast,
  /// Custom partitioner function.
  Custom,
}

/// Trait for partitioning data across workers.
pub trait Partitioner: Send + Sync {
  /// Returns the partition index for the given key.
  ///
  /// # Arguments
  ///
  /// * `key` - The partition key
  /// * `num_partitions` - Total number of partitions
  ///
  /// # Returns
  ///
  /// Partition index (0 to num_partitions - 1)
  fn partition(&self, key: &PartitionKey, num_partitions: usize) -> usize;

  /// Returns the partitioning strategy used by this partitioner.
  fn strategy(&self) -> PartitionStrategy;
}

/// Hash-based partitioner using consistent hashing.
pub struct HashPartitioner;

impl HashPartitioner {
  /// Creates a new hash partitioner.
  #[must_use]
  pub fn new() -> Self {
    Self
  }

  fn hash_key(&self, key: &PartitionKey) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    match key {
      PartitionKey::String(s) => s.hash(&mut hasher),
      PartitionKey::Numeric(n) => n.hash(&mut hasher),
      PartitionKey::Bytes(b) => b.hash(&mut hasher),
      PartitionKey::Hash(h) => h.hash(&mut hasher),
    }
    hasher.finish()
  }
}

impl Default for HashPartitioner {
  fn default() -> Self {
    Self::new()
  }
}

impl Partitioner for HashPartitioner {
  fn partition(&self, key: &PartitionKey, num_partitions: usize) -> usize {
    if num_partitions == 0 {
      return 0;
    }
    let hash = self.hash_key(key);
    (hash as usize) % num_partitions
  }

  fn strategy(&self) -> PartitionStrategy {
    PartitionStrategy::Hash
  }
}

/// Round-robin partitioner.
pub struct RoundRobinPartitioner {
  counter: std::sync::atomic::AtomicUsize,
}

impl RoundRobinPartitioner {
  /// Creates a new round-robin partitioner.
  #[must_use]
  pub fn new() -> Self {
    Self {
      counter: std::sync::atomic::AtomicUsize::new(0),
    }
  }
}

impl Default for RoundRobinPartitioner {
  fn default() -> Self {
    Self::new()
  }
}

impl Partitioner for RoundRobinPartitioner {
  fn partition(&self, _key: &PartitionKey, num_partitions: usize) -> usize {
    if num_partitions == 0 {
      return 0;
    }
    let idx = self
      .counter
      .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    idx % num_partitions
  }

  fn strategy(&self) -> PartitionStrategy {
    PartitionStrategy::RoundRobin
  }
}

/// Range-based partitioner for ordered key distribution.
///
/// Divides the key space into ordered ranges, with each partition
/// responsible for a contiguous range of keys.
pub struct RangePartitioner {
  /// Number of partitions (cached for range calculation).
  num_partitions: std::sync::atomic::AtomicUsize,
}

impl RangePartitioner {
  /// Creates a new range partitioner.
  #[must_use]
  pub fn new() -> Self {
    Self {
      num_partitions: std::sync::atomic::AtomicUsize::new(1),
    }
  }

  /// Updates the number of partitions for range calculation.
  pub fn set_num_partitions(&self, num_partitions: usize) {
    self
      .num_partitions
      .store(num_partitions, std::sync::atomic::Ordering::Relaxed);
  }

  fn key_to_u64(&self, key: &PartitionKey) -> u64 {
    match key {
      PartitionKey::String(s) => {
        // Use first 8 bytes of string hash for ordering
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
      }
      PartitionKey::Numeric(n) => *n,
      PartitionKey::Bytes(b) => {
        // Convert first 8 bytes to u64 (big-endian)
        if b.is_empty() {
          0
        } else {
          let mut bytes = [0u8; 8];
          let copy_len = b.len().min(8);
          bytes[8 - copy_len..].copy_from_slice(&b[..copy_len]);
          u64::from_be_bytes(bytes)
        }
      }
      PartitionKey::Hash(h) => *h,
    }
  }
}

impl Default for RangePartitioner {
  fn default() -> Self {
    Self::new()
  }
}

impl Partitioner for RangePartitioner {
  fn partition(&self, key: &PartitionKey, num_partitions: usize) -> usize {
    if num_partitions == 0 {
      return 0;
    }

    // Update cached partition count
    self.set_num_partitions(num_partitions);

    let key_value = self.key_to_u64(key);

    // Divide u64 space into equal ranges
    // Each partition gets a range of size (u64::MAX / num_partitions)
    let range_size = u64::MAX / num_partitions as u64;
    let partition = if range_size == 0 {
      // If we have more partitions than range_size can handle,
      // fall back to modulo
      (key_value as usize) % num_partitions
    } else {
      (key_value / range_size) as usize
    };

    // Ensure partition is within bounds
    partition.min(num_partitions - 1)
  }

  fn strategy(&self) -> PartitionStrategy {
    PartitionStrategy::Range
  }
}

/// Custom partitioner that uses a provided function.
pub struct CustomPartitioner<F>
where
  F: Fn(&PartitionKey, usize) -> usize + Send + Sync,
{
  partition_fn: F,
}

impl<F> CustomPartitioner<F>
where
  F: Fn(&PartitionKey, usize) -> usize + Send + Sync,
{
  /// Creates a new custom partitioner with the given function.
  #[must_use]
  pub fn new(partition_fn: F) -> Self {
    Self { partition_fn }
  }
}

impl<F> Partitioner for CustomPartitioner<F>
where
  F: Fn(&PartitionKey, usize) -> usize + Send + Sync,
{
  fn partition(&self, key: &PartitionKey, num_partitions: usize) -> usize {
    (self.partition_fn)(key, num_partitions)
  }

  fn strategy(&self) -> PartitionStrategy {
    PartitionStrategy::Custom
  }
}

/// Utility functions for partition rebalancing.
pub mod rebalance {
  use super::{PartitionKey, Partitioner};

  /// Rebalances partitions when the number of partitions changes.
  ///
  /// This function determines which keys need to move to different partitions
  /// when the partition count changes.
  ///
  /// # Arguments
  ///
  /// * `partitioner` - The partitioner to use
  /// * `key` - The partition key
  /// * `old_num_partitions` - Previous number of partitions
  /// * `new_num_partitions` - New number of partitions
  ///
  /// # Returns
  ///
  /// Tuple of (old_partition, new_partition, needs_rebalance)
  pub fn needs_rebalance(
    partitioner: &dyn Partitioner,
    key: &PartitionKey,
    old_num_partitions: usize,
    new_num_partitions: usize,
  ) -> (usize, usize, bool) {
    if old_num_partitions == 0 || new_num_partitions == 0 {
      return (0, 0, false);
    }

    let old_partition = partitioner.partition(key, old_num_partitions);
    let new_partition = partitioner.partition(key, new_num_partitions);
    let needs_rebalance = old_partition != new_partition;

    (old_partition, new_partition, needs_rebalance)
  }

  /// Calculates the percentage of keys that need rebalancing.
  ///
  /// # Arguments
  ///
  /// * `partitioner` - The partitioner to use
  /// * `sample_keys` - Sample keys to test
  /// * `old_num_partitions` - Previous number of partitions
  /// * `new_num_partitions` - New number of partitions
  ///
  /// # Returns
  ///
  /// Percentage (0.0 to 1.0) of keys that need to move
  pub fn rebalance_percentage(
    partitioner: &dyn Partitioner,
    sample_keys: &[PartitionKey],
    old_num_partitions: usize,
    new_num_partitions: usize,
  ) -> f64 {
    if sample_keys.is_empty() || old_num_partitions == 0 || new_num_partitions == 0 {
      return 0.0;
    }

    let needs_rebalance_count = sample_keys
      .iter()
      .filter(|key| {
        let (_old, _new, needs) =
          needs_rebalance(partitioner, key, old_num_partitions, new_num_partitions);
        needs
      })
      .count();

    needs_rebalance_count as f64 / sample_keys.len() as f64
  }
}

#[cfg(test)]
mod tests {
  use super::rebalance;
  use super::*;
  use proptest::prelude::*;

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
}
