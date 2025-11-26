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

  #[test]
  fn test_hash_partitioner() {
    let partitioner = HashPartitioner::new();
    let key = PartitionKey::String("test-key".to_string());
    let partition = partitioner.partition(&key, 5);
    assert!(partition < 5);

    // Same key should always map to same partition
    let partition2 = partitioner.partition(&key, 5);
    assert_eq!(partition, partition2);
  }

  #[test]
  fn test_hash_partitioner_consistency() {
    let partitioner = HashPartitioner::new();
    let key = PartitionKey::String("consistent-key".to_string());

    // Same key should map to same partition regardless of when called
    let p1 = partitioner.partition(&key, 10);
    let p2 = partitioner.partition(&key, 10);
    let p3 = partitioner.partition(&key, 10);

    assert_eq!(p1, p2);
    assert_eq!(p2, p3);
  }

  #[test]
  fn test_round_robin_partitioner() {
    let partitioner = RoundRobinPartitioner::new();
    let key = PartitionKey::String("test".to_string());
    let p1 = partitioner.partition(&key, 3);
    let p2 = partitioner.partition(&key, 3);
    let p3 = partitioner.partition(&key, 3);
    assert_ne!(p1, p2);
    assert_ne!(p2, p3);
    assert!(p1 < 3 && p2 < 3 && p3 < 3);
  }

  #[test]
  fn test_round_robin_even_distribution() {
    let partitioner = RoundRobinPartitioner::new();
    let key = PartitionKey::String("test".to_string());
    let num_partitions = 3;

    // Get 30 partitions
    let partitions: Vec<usize> = (0..30)
      .map(|_| partitioner.partition(&key, num_partitions))
      .collect();

    // Count distribution
    let mut counts = vec![0; num_partitions];
    for &p in &partitions {
      counts[p] += 1;
    }

    // Should be roughly even (10 each, but may vary slightly due to atomic counter)
    for count in counts {
      assert!(
        (9..=11).contains(&count),
        "Distribution should be roughly even"
      );
    }
  }

  #[test]
  fn test_range_partitioner() {
    let partitioner = RangePartitioner::new();

    // Test numeric keys (ordered)
    let key1 = PartitionKey::Numeric(100);
    let key2 = PartitionKey::Numeric(200);
    let key3 = PartitionKey::Numeric(300);

    let p1 = partitioner.partition(&key1, 3);
    let p2 = partitioner.partition(&key2, 3);
    let p3 = partitioner.partition(&key3, 3);

    // All should be valid partitions
    assert!(p1 < 3);
    assert!(p2 < 3);
    assert!(p3 < 3);

    // Same key should map to same partition
    let p1_again = partitioner.partition(&key1, 3);
    assert_eq!(p1, p1_again);
  }

  #[test]
  fn test_range_partitioner_consistency() {
    let partitioner = RangePartitioner::new();
    let key = PartitionKey::Numeric(12345);

    // Same key should map to same partition
    let p1 = partitioner.partition(&key, 10);
    let p2 = partitioner.partition(&key, 10);

    assert_eq!(p1, p2);
  }

  #[test]
  fn test_custom_partitioner() {
    let partitioner = CustomPartitioner::new(|key, num_partitions| match key {
      PartitionKey::Numeric(n) => (*n as usize) % num_partitions,
      _ => 0,
    });

    let key1 = PartitionKey::Numeric(10);
    let key2 = PartitionKey::Numeric(11);

    let p1 = partitioner.partition(&key1, 5);
    let p2 = partitioner.partition(&key2, 5);

    assert_eq!(p1, 0); // 10 % 5 = 0
    assert_eq!(p2, 1); // 11 % 5 = 1
    assert_eq!(partitioner.strategy(), PartitionStrategy::Custom);
  }

  #[test]
  fn test_rebalance_needs_rebalance() {
    let partitioner = HashPartitioner::new();
    let key = PartitionKey::String("test-key".to_string());

    // Check rebalancing when partition count changes
    let (old, new, needs) = rebalance::needs_rebalance(&partitioner, &key, 3, 5);

    assert!(old < 3);
    assert!(new < 5);
    // Rebalancing may or may not be needed depending on hash
    assert!(needs == (old != new));
  }

  #[test]
  fn test_rebalance_hash_partitioner_scaling() {
    let partitioner = HashPartitioner::new();
    let sample_keys: Vec<PartitionKey> = (0..100).map(PartitionKey::Numeric).collect();

    // Scale from 5 to 10 partitions
    let percentage = rebalance::rebalance_percentage(&partitioner, &sample_keys, 5, 10);

    // Hash partitioner should have significant rebalancing when doubling partitions
    // Exact percentage depends on hash distribution, but should be > 0
    assert!(percentage > 0.0);
    assert!(percentage <= 1.0);
  }

  #[test]
  fn test_zero_partitions() {
    let partitioner = HashPartitioner::new();
    let key = PartitionKey::String("test".to_string());

    let p = partitioner.partition(&key, 0);
    assert_eq!(p, 0);
  }

  #[test]
  fn test_single_partition() {
    let partitioner = HashPartitioner::new();
    let key1 = PartitionKey::String("key1".to_string());
    let key2 = PartitionKey::String("key2".to_string());

    let p1 = partitioner.partition(&key1, 1);
    let p2 = partitioner.partition(&key2, 1);

    // All keys should map to partition 0
    assert_eq!(p1, 0);
    assert_eq!(p2, 0);
  }
}
