//! # Memoizing Map Node
//!
//! A transform node that wraps a map function with an optional cache. For deterministic,
//! keyed nodes, repeated inputs with the same key skip recomputation and emit cached output.
//!
//! See [incremental-recomputation.md](../../docs/incremental-recomputation.md) §4.1.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives MapConfig (transformation function)
//! - **Input**: `"in"` - Receives data items to transform
//! - **Output**: `"out"` - Sends transformed data items
//! - **Output**: `"error"` - Sends errors that occur during transformation
//!
//! ## Cache key
//!
//! By default, the cache key is the **identity** of the input (`Arc` pointer). Same allocation
//! = cache hit. For value-based caching (e.g. same integer value = hit), provide a custom
//! [`MemoizeKeyExtractor`].

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use crate::nodes::map_node::MapConfig;
use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use std::any::Any;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;

/// Extracts a cache key from an input item for memoization.
///
/// - **Identity-based (default):** Same `Arc` allocation = cache hit.
/// - **Value-based (custom):** Implement to return a hash for value equality
///   (e.g. `hash(i32)` so that `Arc::new(5)` and `Arc::new(5)` hit the same cache).
pub trait MemoizeKeyExtractor: Send + Sync {
  /// Returns a cache key for the item. If `None`, the item is not cached.
  fn extract_key(&self, item: &Arc<dyn Any + Send + Sync>) -> Option<u64>;
}

/// Identity-based key: same `Arc` allocation = same key.
#[derive(Clone, Copy, Default)]
pub struct IdentityKeyExtractor;

impl MemoizeKeyExtractor for IdentityKeyExtractor {
  fn extract_key(&self, item: &Arc<dyn Any + Send + Sync>) -> Option<u64> {
    Some(Arc::as_ptr(item) as *const () as u64)
  }
}

/// Value-based key extractor for types implementing `Hash`.
///
/// Use when you want cache hits for equal values (e.g. two `Arc::new(42i32)`).
pub struct HashKeyExtractor;

impl MemoizeKeyExtractor for HashKeyExtractor {
  fn extract_key(&self, item: &Arc<dyn Any + Send + Sync>) -> Option<u64> {
    use std::collections::hash_map::DefaultHasher;
    if let Some(h) = item.downcast_ref::<i32>() {
      let mut hasher = DefaultHasher::new();
      h.hash(&mut hasher);
      return Some(hasher.finish());
    }
    if let Some(h) = item.downcast_ref::<i64>() {
      let mut hasher = DefaultHasher::new();
      h.hash(&mut hasher);
      return Some(hasher.finish());
    }
    if let Some(h) = item.downcast_ref::<u64>() {
      let mut hasher = DefaultHasher::new();
      h.hash(&mut hasher);
      return Some(hasher.finish());
    }
    if let Some(h) = item.downcast_ref::<String>() {
      let mut hasher = DefaultHasher::new();
      h.hash(&mut hasher);
      return Some(hasher.finish());
    }
    None
  }
}

/// A map node with memoization: caches output keyed by input for deterministic,
/// keyed transformations. Skips recomputation when the same key is seen again.
pub struct MemoizingMapNode {
  base: BaseNode,
  current_config: Arc<Mutex<Option<Arc<MapConfig>>>>,
  cache: Arc<Mutex<HashMap<u64, Arc<dyn Any + Send + Sync>>>>,
  key_extractor: Arc<dyn MemoizeKeyExtractor>,
}

impl MemoizingMapNode {
  /// Creates a new MemoizingMapNode with identity-based caching.
  pub fn new(name: String) -> Self {
    Self::with_key_extractor(name, Arc::new(IdentityKeyExtractor))
  }

  /// Creates a MemoizingMapNode with a custom key extractor for value-based caching.
  pub fn with_key_extractor(name: String, key_extractor: Arc<dyn MemoizeKeyExtractor>) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec!["configuration".to_string(), "in".to_string()],
        vec!["out".to_string(), "error".to_string()],
      ),
      current_config: Arc::new(Mutex::new(None)),
      cache: Arc::new(Mutex::new(HashMap::new())),
      key_extractor,
    }
  }

  /// Clears the cache. Call when inputs are replayed or cache should be invalidated.
  pub async fn clear_cache(&self) {
    self.cache.lock().await.clear();
  }
}

#[async_trait]
impl Node for MemoizingMapNode {
  fn name(&self) -> &str {
    self.base.name()
  }

  fn set_name(&mut self, name: &str) {
    self.base.set_name(name);
  }

  fn input_port_names(&self) -> &[String] {
    self.base.input_port_names()
  }

  fn output_port_names(&self) -> &[String] {
    self.base.output_port_names()
  }

  fn has_input_port(&self, name: &str) -> bool {
    self.base.has_input_port(name)
  }

  fn has_output_port(&self, name: &str) -> bool {
    self.base.has_output_port(name)
  }

  fn execute(
    &self,
    mut inputs: InputStreams,
  ) -> Pin<
    Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>,
  > {
    let config_stream = inputs
      .remove("configuration")
      .ok_or("Missing 'configuration' input");
    let data_stream = inputs.remove("in").ok_or("Missing 'in' input");
    let current_config = Arc::clone(&self.current_config);
    let cache = Arc::clone(&self.cache);
    let key_extractor = Arc::clone(&self.key_extractor);

    Box::pin(async move {
      let config_stream = config_stream?;
      let data_stream = data_stream?;

      let config_stream = config_stream.map(|item| (MessageType::Config, item));
      let data_stream = data_stream.map(|item| (MessageType::Data, item));
      let merged = stream::select(config_stream, data_stream);

      let (out_tx, out_rx) = tokio::sync::mpsc::channel(64);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(64);

      let mut current_config_opt: Option<Arc<MapConfig>> = None;

      tokio::spawn(async move {
        let mut merged = merged;
        while let Some((msg_type, item)) = merged.next().await {
          match msg_type {
            MessageType::Config => {
              let cfg_opt = item
                .clone()
                .downcast::<Arc<MapConfig>>()
                .ok()
                .map(|a| (*a).clone())
                .or_else(|| item.downcast::<MapConfig>().ok());
              if let Some(cfg) = cfg_opt {
                current_config_opt = Some(cfg.clone());
                *current_config.lock().await = Some(cfg);
              } else {
                let _ = error_tx
                  .send(Arc::new(format!(
                    "Invalid config type, expected {}",
                    std::any::type_name::<MapConfig>()
                  )) as Arc<dyn Any + Send + Sync>)
                  .await;
              }
            }
            MessageType::Data => {
              let cfg = match &current_config_opt {
                Some(c) => c.clone(),
                None => {
                  let _ =
                    error_tx
                      .send(
                        Arc::new("No configuration set".to_string()) as Arc<dyn Any + Send + Sync>
                      )
                      .await;
                  continue;
                }
              };

              let key = key_extractor.extract_key(&item);

              if let Some(k) = key
                && let Some(cached) = cache.lock().await.get(&k)
              {
                let _ = out_tx.send(cached.clone()).await;
                continue;
              }

              match cfg.apply(item).await {
                Ok(output) => {
                  if let Some(k) = key {
                    cache.lock().await.insert(k, output.clone());
                  }
                  let _ = out_tx.send(output).await;
                }
                Err(e) => {
                  let _ = error_tx
                    .send(Arc::new(e) as Arc<dyn Any + Send + Sync>)
                    .await;
                }
              }
            }
          }
        }
      });

      let mut outputs = HashMap::new();
      outputs.insert(
        "out".to_string(),
        Box::pin(ReceiverStream::new(out_rx))
          as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );
      outputs.insert(
        "error".to_string(),
        Box::pin(ReceiverStream::new(error_rx))
          as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );

      Ok(outputs)
    })
  }
}

enum MessageType {
  Config,
  Data,
}
