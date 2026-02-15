//! # Partitioning Contract
//!
//! API for declaring that a graph or input is **partitioned by key**. Used when
//! running N graph instances (shards); each instance sees only the subset of data
//! whose partition key maps to that shard.
//!
//! See [cluster-sharding.md](../docs/cluster-sharding.md) for design and usage.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use streamweave::partitioning::{PartitionKey, PartitionKeyExtractor, PartitioningConfig, partition_by_key};
//! use std::sync::Arc;
//!
//! // Create a config that extracts user_id from payloads
//! let config = partition_by_key(|payload: Arc<dyn std::any::Any + Send + Sync>| {
//!     Box::pin(async move {
//!         if let Ok(map) = payload.downcast::<std::collections::HashMap<String, i64>>() {
//!             Ok(PartitionKey::from(map.get("user_id").copied().unwrap_or(0).to_string()))
//!         } else {
//!             Err("Expected HashMap with user_id".to_string())
//!         }
//!     })
//! });
//!
//! // Pass config when building a sharded graph; run N instances with different key ranges
//! ```

use async_trait::async_trait;
use std::sync::Arc;

/// Partition key that determines which shard handles a record.
///
/// Typically a string representation of the key (e.g. `user_id`, `(tenant_id, session_id)`).
/// The driver hashes or maps this to a shard index when routing.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PartitionKey(pub String);

impl PartitionKey {
  /// Creates a new partition key from a string.
  pub fn new(s: String) -> Self {
    Self(s)
  }

  /// Returns the key as a string slice.
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

impl From<String> for PartitionKey {
  fn from(s: String) -> Self {
    Self(s)
  }
}

impl From<&str> for PartitionKey {
  fn from(s: &str) -> Self {
    Self(s.to_string())
  }
}

/// Extracts a partition key from a payload.
///
/// Used to declare "this input is partitioned by key K." The same logical graph
/// is instantiated N times; each instance receives only records whose key maps to
/// that shard.
#[async_trait]
pub trait PartitionKeyExtractor: Send + Sync {
  /// Extracts the partition key from a payload.
  ///
  /// Returns an error if the payload cannot be parsed (e.g. missing field).
  async fn extract_key(
    &self,
    payload: Arc<dyn std::any::Any + Send + Sync>,
  ) -> Result<PartitionKey, String>;
}

/// Configuration for partitioning a graph or input by key.
///
/// Declares that data is partitioned by the given key extractor. When running
/// N graph instances (shards), each instance is assigned a key range or consistent-hash
/// range; input is routed so that each instance only sees keys in its range.
#[derive(Clone)]
pub struct PartitioningConfig {
  /// Extractor that derives the partition key from each payload.
  pub key_extractor: Arc<dyn PartitionKeyExtractor>,
}

impl PartitioningConfig {
  /// Creates a new partitioning config with the given key extractor.
  pub fn new(key_extractor: Arc<dyn PartitionKeyExtractor>) -> Self {
    Self { key_extractor }
  }
}

/// Wrapper that implements `PartitionKeyExtractor` for async closures.
struct PartitionKeyExtractorWrapper<F> {
  function: F,
}

#[async_trait]
impl<F> PartitionKeyExtractor for PartitionKeyExtractorWrapper<F>
where
  F: Fn(
      Arc<dyn std::any::Any + Send + Sync>,
    )
      -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<PartitionKey, String>> + Send>>
    + Send
    + Sync,
{
  async fn extract_key(
    &self,
    payload: Arc<dyn std::any::Any + Send + Sync>,
  ) -> Result<PartitionKey, String> {
    (self.function)(payload).await
  }
}

/// Creates a `PartitioningConfig` from an async closure.
pub fn partition_by_key<F, Fut>(function: F) -> PartitioningConfig
where
  F: Fn(Arc<dyn std::any::Any + Send + Sync>) -> Fut + Send + Sync + 'static,
  Fut: std::future::Future<Output = Result<PartitionKey, String>> + Send + 'static,
{
  PartitioningConfig::new(Arc::new(PartitionKeyExtractorWrapper {
    function: move |p| {
      Box::pin(function(p))
        as std::pin::Pin<Box<dyn std::future::Future<Output = Result<PartitionKey, String>> + Send>>
    },
  }))
}
