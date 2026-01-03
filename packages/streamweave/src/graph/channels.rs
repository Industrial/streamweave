//! # Type-Erased Zero-Copy Channels for Message<T>
//!
//! This module provides type-erased channel types that support both serialized
//! `Bytes` (for distributed execution) and zero-copy `Arc<Message<T>>` (for in-process execution).
//! All data flowing through graph channels is wrapped in `Message<T>` to enable
//! end-to-end traceability and metadata preservation.
//!
//! ## Architecture
//!
//! The `ChannelItem` enum allows the graph executor to work with type-erased nodes
//! (`Box<dyn NodeTrait>`) while maintaining type safety at the node level. Nodes
//! extract the appropriate type based on `ExecutionMode`. All items are wrapped
//! in `Message<T>` to preserve message IDs and metadata.
//!
//! ## Usage
//!
//! ```rust
//! use streamweave::graph::channels::{ChannelItem, TypeErasedSender, TypeErasedReceiver};
//! use streamweave::message::{Message, wrap_message};
//! use bytes::Bytes;
//! use std::sync::Arc;
//!
//! // Create type-erased channels
//! let (sender, mut receiver): (TypeErasedSender, TypeErasedReceiver) =
//!     tokio::sync::mpsc::channel(1024);
//!
//! // Send Bytes containing serialized Message<T> (distributed mode)
//! let msg = wrap_message(42i32);
//! // Message will be serialized to Bytes before sending
//! sender.send(ChannelItem::Bytes(/* serialized Message<i32> */)).await?;
//!
//! // Send Arc<Message<T>> (in-process mode)
//! let msg = wrap_message(42i32);
//! let msg_arc = Arc::new(msg);
//! sender.send(ChannelItem::Arc(msg_arc.clone())).await?;
//!
//! // Receive and extract Message<T>
//! if let Some(item) = receiver.recv().await {
//!     match item {
//!         ChannelItem::Bytes(bytes) => {
//!             // Deserialize to Message<T>
//!             // let msg: Message<i32> = deserialize(bytes)?;
//!         }
//!         ChannelItem::Arc(arc) => {
//!             // Downcast to Arc<Message<T>>
//!             if let Ok(msg_arc) = Arc::downcast::<Message<i32>>(arc) {
//!                 // Use msg_arc
//!             }
//!         }
//!     }
//! }
//! ```

use super::shared_memory_channel::SharedMemoryRef;
use bytes::Bytes;
use std::any::Any;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Type-erased channel item that can hold either serialized `Message<T>` (distributed)
/// or `Arc<Message<T>>` (in-process).
///
/// This enum enables type erasure at the executor level while allowing type-safe
/// extraction at the node level. The executor stores channels as `TypeErasedSender`/`TypeErasedReceiver`,
/// and nodes extract the appropriate type based on `ExecutionMode`. All data is wrapped
/// in `Message<T>` to preserve message IDs and metadata throughout the graph.
///
/// # Variants
///
/// - `Bytes`: Serialized `Message<T>` for distributed execution
/// - `Arc`: Type-erased `Arc<Message<T>>` for zero-copy in-process execution
/// - `SharedMemory`: Reference to `Message<T>` in shared memory segment (ultra-high performance)
///
/// # Example
///
/// ```rust
/// use streamweave::graph::channels::ChannelItem;
/// use streamweave::message::{Message, wrap_message};
/// use bytes::Bytes;
/// use std::sync::Arc;
///
/// // Create Bytes variant (contains serialized Message<T>)
/// // In practice, Message<T> would be serialized to Bytes
/// let bytes_item = ChannelItem::Bytes(Bytes::from("serialized Message<T>"));
///
/// // Create Arc variant (contains Arc<Message<T>>)
/// let msg = wrap_message(42i32);
/// let arc_item = ChannelItem::Arc(Arc::new(msg));
///
/// // Extract Bytes (will need deserialization to Message<T>)
/// if let ChannelItem::Bytes(b) = bytes_item {
///     // Deserialize to Message<T>
///     // let msg: Message<i32> = deserialize(b)?;
/// }
///
/// // Extract and downcast Arc to Arc<Message<T>>
/// if let ChannelItem::Arc(arc) = arc_item {
///     if let Ok(msg_arc) = Arc::downcast::<Message<i32>>(arc) {
///         assert_eq!(*msg_arc.payload(), 42);
///     }
/// }
/// ```
#[derive(Clone, Debug)]
pub enum ChannelItem {
  /// Serialized `Message<T>` as bytes for distributed execution.
  ///
  /// This variant is used when `ExecutionMode::Distributed` is active.
  /// `Message<T>` is serialized to `Bytes` before transmission, preserving
  /// message ID and metadata.
  Bytes(Bytes),
  /// Type-erased `Arc<Message<T>>` for zero-copy in-process execution.
  ///
  /// This variant is used when `ExecutionMode::InProcess` is active.
  /// `Message<T>` is wrapped in `Arc<dyn Any + Send + Sync>` to enable zero-copy
  /// sharing between nodes in the same process while preserving message structure.
  Arc(Arc<dyn Any + Send + Sync>),
  /// Reference to `Message<T>` in shared memory segment.
  ///
  /// This variant is used when `ExecutionMode::InProcess { use_shared_memory: true }` is active.
  /// `Message<T>` is stored in OS-native shared memory segments for ultra-high performance.
  /// Only a lightweight reference is passed through channels.
  SharedMemory(SharedMemoryRef),
}

impl ChannelItem {
  /// Extract a reference to `Bytes` if this is a `Bytes` variant.
  ///
  /// # Returns
  ///
  /// `Some(&Bytes)` if this is a `Bytes` variant, `None` otherwise.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::graph::channels::ChannelItem;
  /// use bytes::Bytes;
  ///
  /// let item = ChannelItem::Bytes(Bytes::from("hello"));
  /// if let Some(bytes) = item.as_bytes() {
  ///     assert_eq!(bytes, &Bytes::from("hello"));
  /// }
  /// ```
  #[must_use]
  pub fn as_bytes(&self) -> Option<&Bytes> {
    match self {
      ChannelItem::Bytes(b) => Some(b),
      _ => None,
    }
  }

  /// Extract `Bytes` by consuming this `ChannelItem`.
  ///
  /// # Returns
  ///
  /// `Ok(Bytes)` if this is a `Bytes` variant, `Err(self)` otherwise.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::graph::channels::ChannelItem;
  /// use bytes::Bytes;
  ///
  /// let item = ChannelItem::Bytes(Bytes::from("hello"));
  /// match item.into_bytes() {
  ///     Ok(bytes) => assert_eq!(bytes, Bytes::from("hello")),
  ///     Err(_) => panic!("Expected Bytes variant"),
  /// }
  /// ```
  pub fn into_bytes(self) -> Result<Bytes, Self> {
    match self {
      ChannelItem::Bytes(b) => Ok(b),
      other => Err(other),
    }
  }

  /// Extract and downcast `Arc<Message<T>>` if this is an `Arc` variant.
  ///
  /// # Type Parameters
  ///
  /// * `T` - The payload type of the message. Must be `'static + Send + Sync`.
  ///
  /// # Returns
  ///
  /// `Ok(Arc<Message<T>>)` if this is an `Arc` variant and the type matches, `Err(self)` otherwise.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::graph::channels::ChannelItem;
  /// use streamweave::message::{Message, wrap_message};
  /// use std::sync::Arc;
  ///
  /// let msg = wrap_message(42i32);
  /// let item = ChannelItem::Arc(Arc::new(msg));
  /// match item.downcast_message_arc::<i32>() {
  ///     Ok(msg_arc) => assert_eq!(*msg_arc.payload(), 42),
  ///     Err(_) => panic!("Expected Arc<Message<i32>>"),
  /// }
  /// ```
  pub fn downcast_message_arc<T: 'static + Send + Sync>(
    self,
  ) -> Result<Arc<crate::Message<T>>, Self> {
    match self {
      ChannelItem::Arc(arc) => Arc::downcast(arc).map_err(ChannelItem::Arc),
      other => Err(other),
    }
  }

  /// Extract and downcast `Arc<T>` if this is an `Arc` variant (legacy method for backward compatibility).
  ///
  /// # Type Parameters
  ///
  /// * `T` - The type to downcast to. Must be `'static + Send + Sync`.
  ///
  /// # Returns
  ///
  /// `Ok(Arc<T>)` if this is an `Arc` variant and the type matches, `Err(self)` otherwise.
  ///
  /// # Note
  ///
  /// This method is provided for backward compatibility. In the new Message-based architecture,
  /// you should use `downcast_message_arc::<T>()` to get `Arc<Message<T>>`.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::graph::channels::ChannelItem;
  /// use std::sync::Arc;
  ///
  /// let item = ChannelItem::Arc(Arc::new(42i32));
  /// match item.downcast_arc::<i32>() {
  ///     Ok(arc) => assert_eq!(*arc, 42),
  ///     Err(_) => panic!("Expected Arc<i32>"),
  /// }
  /// ```
  pub fn downcast_arc<T: 'static + Send + Sync>(self) -> Result<Arc<T>, Self> {
    match self {
      ChannelItem::Arc(arc) => Arc::downcast(arc).map_err(ChannelItem::Arc),
      other => Err(other),
    }
  }

  /// Check if this is a `Bytes` variant.
  ///
  /// # Returns
  ///
  /// `true` if this is a `Bytes` variant, `false` otherwise.
  #[must_use]
  pub fn is_bytes(&self) -> bool {
    matches!(self, ChannelItem::Bytes(_))
  }

  /// Check if this is an `Arc` variant.
  ///
  /// # Returns
  ///
  /// `true` if this is an `Arc` variant, `false` otherwise.
  #[must_use]
  pub fn is_arc(&self) -> bool {
    matches!(self, ChannelItem::Arc(_))
  }
}

/// Type-erased channel sender.
///
/// This type alias represents a sender that can send `ChannelItem` instances.
/// It's used by the graph executor to store channels in a type-erased way.
///
/// # Example
///
/// ```rust
/// use streamweave::graph::channels::{TypeErasedSender, ChannelItem};
/// use bytes::Bytes;
///
/// let (sender, _receiver): (TypeErasedSender, _) = tokio::sync::mpsc::channel(1024);
/// sender.send(ChannelItem::Bytes(Bytes::from("hello"))).await?;
/// ```
pub type TypeErasedSender = mpsc::Sender<ChannelItem>;

/// Type-erased channel receiver.
///
/// This type alias represents a receiver that can receive `ChannelItem` instances.
/// It's used by the graph executor to store channels in a type-erased way.
///
/// # Example
///
/// ```rust
/// use streamweave::graph::channels::{TypeErasedReceiver, ChannelItem};
///
/// let (_sender, mut receiver): (_, TypeErasedReceiver) = tokio::sync::mpsc::channel(1024);
/// if let Some(item) = receiver.recv().await {
///     match item {
///         ChannelItem::Bytes(bytes) => {
///             // Handle serialized data
///         }
///         ChannelItem::Arc(arc) => {
///             // Downcast to specific type
///         }
///     }
/// }
/// ```
pub type TypeErasedReceiver = mpsc::Receiver<ChannelItem>;
