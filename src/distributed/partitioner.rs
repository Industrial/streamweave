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

#[cfg(test)]
mod tests {
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
}
