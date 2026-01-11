//! Key-based router for routing items based on key extraction.
//!
//! This module provides [`KeyBasedRouter`], a router that routes items based on a key
//! extraction function. Items with the same key go to the same port, making it ideal
//! for partitioning data by key. It implements [`OutputRouter`] for use in
//! StreamWeave graphs.
//!
//! # Overview
//!
//! [`KeyBasedRouter`] is useful for partitioning data in graph-based pipelines.
//! It extracts a key from each item and routes items with the same key to the
//! same output port, ensuring deterministic routing and enabling parallel
//! processing of different partitions.
//!
//! # Key Concepts
//!
//! - **Key Extraction**: Uses a key extraction function to determine routing
//! - **Deterministic Routing**: Items with the same key always go to the same port
//! - **Hash-Based Distribution**: Uses hash-based distribution for efficient routing
//! - **Multiple Output Ports**: Supports multiple output ports for partitioning
//!
//! # Core Types
//!
//! - **[`KeyBasedRouter<O, K>`]**: Router that routes items based on a key
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::KeyBasedRouter;
//!
//! // Route items based on a key extracted from the item
//! let router = KeyBasedRouter::new(
//!     |x: &i32| *x % 3,  // Key extraction function
//!     vec![0, 1, 2]      // Output ports
//! );
//! ```
//!
//! ## With Explicit Key-to-Port Mapping
//!
//! ```rust
//! use streamweave::graph::nodes::KeyBasedRouter;
//!
//! // Create router with explicit key-to-port mapping
//! let mut router = KeyBasedRouter::new_with_mapping(
//!     |item: &MyStruct| item.category.clone(),
//!     vec![0, 1, 2],
//!     |key: &String| match key.as_str() {
//!         "A" => 0,
//!         "B" => 1,
//!         _ => 2,
//!     }
//! );
//! ```
//!
//! # Design Decisions
//!
//! - **Hash-Based Routing**: Uses hash-based distribution for efficient routing
//! - **Deterministic**: Items with the same key always route to the same port
//! - **Key Function**: Uses a closure for flexible key extraction
//! - **Router Trait**: Implements `OutputRouter` for integration with graph system
//!
//! # Integration with StreamWeave
//!
//! [`KeyBasedRouter`] implements the [`OutputRouter`] trait and can be used in any
//! StreamWeave graph. It routes items to different output ports based on their
//! extracted keys, enabling partitioning and parallel processing patterns.

use crate::graph::router::OutputRouter;
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;

/// A router that routes items based on a key extraction function.
///
/// This router extracts a key from each item and routes items with the same
/// key to the same port. This is useful for partitioning data by key.
///
/// # Example
///
/// ```rust
/// use crate::graph::routers::KeyBasedRouter;
/// use futures::stream;
/// use std::pin::Pin;
///
/// let mut router = KeyBasedRouter::new(
///     |x: &i32| *x % 3,  // Key function
///     vec![0, 1, 2],      // Output ports
/// );
/// let input_stream: Pin<Box<dyn Stream<Item = i32> + Send>> =
///     Box::pin(stream::iter(vec![1, 2, 3, 4, 5, 6]));
///
/// let output_streams = router.route_stream(input_stream).await;
/// // Items with key % 3 == 0 go to port 0
/// // Items with key % 3 == 1 go to port 1
/// // Items with key % 3 == 2 go to port 2
/// ```
pub struct KeyBasedRouter<O, K> {
  /// The output port indices this router manages
  output_ports: Vec<usize>,
  /// Function to extract key from items
  key_fn: Arc<dyn Fn(&O) -> K + Send + Sync>,
  /// Mapping from keys to port indices
  key_to_port: HashMap<K, usize>,
}

impl<O, K> KeyBasedRouter<O, K>
where
  K: Hash + Eq + Clone + Send + Sync + 'static,
{
  /// Creates a new KeyBasedRouter with the specified key function and ports.
  ///
  /// # Arguments
  ///
  /// * `key_fn` - Function to extract a key from items
  /// * `output_ports` - Vector of port indices to route to
  ///
  /// # Returns
  ///
  /// A new `KeyBasedRouter` instance.
  ///
  /// # Note
  ///
  /// Keys are mapped to ports using a hash-based distribution. Items with the
  /// same key will always go to the same port.
  pub fn new<F>(key_fn: F, output_ports: Vec<usize>) -> Self
  where
    F: Fn(&O) -> K + Send + Sync + 'static,
  {
    let key_to_port = HashMap::new();
    Self {
      output_ports,
      key_fn: Arc::new(key_fn),
      key_to_port,
    }
  }

  /// Creates a new KeyBasedRouter with explicit key-to-port mapping.
  ///
  /// # Arguments
  ///
  /// * `key_fn` - Function to extract a key from items
  /// * `key_to_port` - Explicit mapping from keys to port indices
  /// * `output_ports` - Vector of port indices (for validation)
  ///
  /// # Returns
  ///
  /// A new `KeyBasedRouter` instance with explicit key mapping.
  pub fn with_mapping<F>(
    key_fn: F,
    key_to_port: HashMap<K, usize>,
    output_ports: Vec<usize>,
  ) -> Self
  where
    F: Fn(&O) -> K + Send + Sync + 'static,
  {
    Self {
      output_ports,
      key_fn: Arc::new(key_fn),
      key_to_port,
    }
  }
}

#[async_trait]
impl<O, K> OutputRouter<O> for KeyBasedRouter<O, K>
where
  O: Send + Sync + 'static,
  K: Hash + Eq + Clone + Send + Sync + 'static,
{
  async fn route_stream(
    &mut self,
    stream: Pin<Box<dyn Stream<Item = O> + Send>>,
  ) -> Vec<(String, Pin<Box<dyn Stream<Item = O> + Send>>)> {
    if self.output_ports.is_empty() {
      return Vec::new();
    }

    // Create channels for each output port
    let mut senders = Vec::new();
    let mut receivers = Vec::new();

    for _ in &self.output_ports {
      let (tx, rx) = mpsc::channel(16);
      senders.push(tx);
      receivers.push(rx);
    }

    // Spawn task to route items based on key
    let mut input_stream = stream;
    let senders_clone = senders.clone();
    let key_fn = self.key_fn.clone();
    let num_ports = self.output_ports.len();
    let key_to_port = self.key_to_port.clone();

    tokio::spawn(async move {
      while let Some(item) = input_stream.next().await {
        // Extract key
        let key = key_fn(&item);

        // Determine port (use explicit mapping or hash-based)
        let port_idx = if let Some(&port) = key_to_port.get(&key) {
          port
        } else {
          // Hash-based distribution if no explicit mapping
          let mut hasher = std::collections::hash_map::DefaultHasher::new();
          key.hash(&mut hasher);
          (hasher.finish() as usize) % num_ports
        };

        if port_idx < senders_clone.len() && senders_clone[port_idx].send(item).await.is_err() {
          // Receiver dropped, continue
        }
      }
    });

    // Create streams from receivers
    let mut output_streams: Vec<(String, Pin<Box<dyn Stream<Item = O> + Send>>)> = Vec::new();
    for (idx, &_port) in self.output_ports.iter().enumerate() {
      let port_name = if idx == 0 {
        "out".to_string()
      } else {
        format!("out_{}", idx)
      };
      let mut rx = receivers.remove(0);
      let stream = Box::pin(async_stream::stream! {
        while let Some(item) = rx.recv().await {
          yield item;
        }
      });
      output_streams.push((port_name, stream));
    }

    output_streams
  }

  fn output_port_names(&self) -> Vec<String> {
    // Generate port names based on number of ports
    (0..self.output_ports.len())
      .map(|i| {
        if i == 0 {
          "out".to_string()
        } else {
          format!("out_{}", i)
        }
      })
      .collect()
  }
}
