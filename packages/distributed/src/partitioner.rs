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
