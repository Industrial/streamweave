//! # Graph Node Types
//!
//! This module provides node types that wrap existing Producer, Transformer, and
//! Consumer traits with explicit port tuple information. Nodes use generic type
//! parameters for compile-time specialization.
//!
//! ## Zero-Copy Node Architecture
//!
//! Nodes store their components internally as `Arc<tokio::sync::Mutex<T>>` to enable
//! zero-copy node sharing. This eliminates the `Clone` requirement for producers,
//! transformers, and consumers, allowing control flow transformers and other
//! non-cloneable components to be used in graphs.
//!
//! When nodes are executed, the `Arc` is cloned (zero-copy) and the mutex is locked
//! only when needed. This provides both zero-copy semantics and thread-safe access
//! to node components.
//!
//! ## Example
//!
//! ```rust
//! use streamweave::graph::node::{ProducerNode, TransformerNode, ConsumerNode};
//! use streamweave_array::ArrayProducer;
//! use streamweave_transformers::MapTransformer;
//! use streamweave_vec::VecConsumer;
//!
//! // Producer with single output (does not need to implement Clone)
//! let producer = ProducerNode::new(
//!     "source".to_string(),
//!     ArrayProducer::new(vec![1, 2, 3]),
//! );
//!
//! // Transformer with single input and output (does not need to implement Clone)
//! let transformer = TransformerNode::new(
//!     "mapper".to_string(),
//!     MapTransformer::new(|x: i32| x * 2),
//! );
//!
//! // Consumer with single input (does not need to implement Clone)
//! let consumer = ConsumerNode::new(
//!     "sink".to_string(),
//!     VecConsumer::new(),
//! );
//! ```

use crate::channels::{ChannelItem, TypeErasedReceiver, TypeErasedSender};
use crate::compression::Compression;
use crate::execution::CompressionAlgorithm;
use crate::serialization::serialize;
use crate::traits::{NodeKind, NodeTrait};
use crate::zero_copy::ZeroCopyTransformer;
use async_stream::stream;
use futures::StreamExt;
use serde::{Serialize, de::DeserializeOwned};
use std::any::Any;
use std::borrow::Cow;
use std::marker::PhantomData;
use std::pin::{Pin, pin};
use std::sync::Arc;
use streamweave::Consumer;
use streamweave::Input;
use streamweave::Producer;
use streamweave::Transformer;
use streamweave::port::{GetPort, PortList};
use tokio::sync::Mutex;
use tracing::{error, warn};

// Import helper traits for default port types
use streamweave::ConsumerPorts;
use streamweave::ProducerPorts;
use streamweave::TransformerPorts;

/// Trait for validating that a ProducerNode's output ports match the producer's output type.
///
/// This trait ensures type safety by requiring that the Outputs port tuple matches
/// the producer's Output type. For single-port producers, this means `Outputs = (P::Output,)`.
pub trait ValidateProducerPorts<P, Outputs>
where
  P: Producer,
  P::Output: std::fmt::Debug + Clone + Send + Sync,
  Outputs: PortList,
{
}

/// Trait for validating that a TransformerNode's ports match the transformer's input/output types.
///
/// This trait ensures type safety by requiring that the Inputs and Outputs port tuples
/// match the transformer's Input and Output types respectively.
pub trait ValidateTransformerPorts<T, Inputs, Outputs>
where
  T: Transformer,
  T::Input: std::fmt::Debug + Clone + Send + Sync,
  T::Output: std::fmt::Debug + Clone + Send + Sync,
  Inputs: PortList,
  Outputs: PortList,
{
}

/// Trait for validating that a ConsumerNode's input ports match the consumer's input type.
///
/// This trait ensures type safety by requiring that the Inputs port tuple matches
/// the consumer's Input type. For single-port consumers, this means `Inputs = (C::Input,)`.
pub trait ValidateConsumerPorts<C, Inputs>
where
  C: Consumer,
  C::Input: std::fmt::Debug + Clone + Send + Sync,
  Inputs: PortList,
{
}

// Implement validation for producer: first output port must match P::Output
// For single-port case: Outputs = (P::Output,)
impl<P, Outputs> ValidateProducerPorts<P, Outputs> for ()
where
  P: Producer,
  P::Output: std::fmt::Debug + Clone + Send + Sync,
  Outputs: PortList,
  Outputs: GetPort<0, Type = P::Output>,
{
}

// Implement validation for transformer: first input/output ports must match T::Input/T::Output
// For single-port case: Inputs = (T::Input,), Outputs = (T::Output,)
impl<T, Inputs, Outputs> ValidateTransformerPorts<T, Inputs, Outputs> for ()
where
  T: Transformer,
  T::Input: std::fmt::Debug + Clone + Send + Sync,
  T::Output: std::fmt::Debug + Clone + Send + Sync,
  Inputs: PortList,
  Outputs: PortList,
  Inputs: GetPort<0, Type = T::Input>,
  Outputs: GetPort<0, Type = T::Output>,
{
}

// Implement validation for consumer: first input port must match C::Input
// For single-port case: Inputs = (C::Input,)
impl<C, Inputs> ValidateConsumerPorts<C, Inputs> for ()
where
  C: Consumer,
  C::Input: std::fmt::Debug + Clone + Send + Sync,
  Inputs: PortList,
  Inputs: GetPort<0, Type = C::Input>,
{
}

/// A wrapper type that carries type information for stream conversion.
///
/// This type wraps a `Pin<Box<dyn Stream>>` along with phantom type information
/// from the transformer type `T`. This allows us to convert the trait object stream
/// to `T::InputStream` in a type-safe manner.
///
/// This is Option 6 (Phantom Type Pattern with Conversion Trait) from the architectural
/// options. The wrapper ensures type information flows through the system, enabling
/// safe conversion to the associated type.
struct StreamWrapper<T: Input> {
  stream: Pin<Box<dyn futures::Stream<Item = <T as Input>::Input> + Send>>,
  _phantom: PhantomData<T>,
}

impl<T: Input> StreamWrapper<T>
where
  <T as Input>::Input: DeserializeOwned + Send + Sync + Clone + 'static,
{
  /// Creates a new `StreamWrapper` from a type-erased channel receiver in distributed mode.
  ///
  /// # Arguments
  ///
  /// * `receiver` - Channel receiver that yields `ChannelItem::Bytes`
  /// * `compression` - Optional compression algorithm if compression is enabled
  /// * `batching` - Whether batching is enabled (batches need to be deserialized)
  ///
  /// # Returns
  ///
  /// A new `StreamWrapper` that can be converted to `T::InputStream`.
  fn from_receiver(
    mut receiver: TypeErasedReceiver,
    compression: Option<CompressionAlgorithm>,
    batching: bool,
  ) -> Self {
    let stream = Box::pin(stream! {
      while let Some(channel_item) = receiver.recv().await {
        match channel_item {
          ChannelItem::Bytes(bytes) => {
            // Distributed mode: decompress if needed, then deserialize
            let decompressed_bytes = if let Some(algorithm) = compression {
              // Create decompressor (level doesn't matter for decompression, but we need to match the algorithm)
              let decompressor: Box<dyn Compression> = match algorithm {
                CompressionAlgorithm::Gzip { level } => {
                  Box::new(crate::compression::GzipCompression::new(level))
                }
                CompressionAlgorithm::Zstd { level } => {
                  Box::new(crate::compression::ZstdCompression::new(level))
                }
              };

              // Decompress in blocking task
              match tokio::task::spawn_blocking(move || decompressor.decompress(&bytes)).await {
                Ok(Ok(decompressed)) => decompressed,
                Ok(Err(e)) => {
                  // Handle corrupted data gracefully - log and skip
                  let is_corrupted = matches!(e, crate::compression::CompressionError::CorruptedData);
                  if is_corrupted {
                    warn!(
                      "Corrupted compressed data detected, skipping item"
                    );
                  } else {
                    warn!(
                      error = %e,
                      "Failed to decompress input item, skipping"
                    );
                  }
                  continue;
                }
                Err(e) => {
                  warn!(
                    error = %e,
                    "Decompression task failed, skipping"
                  );
                  continue;
                }
              }
            } else {
              bytes
            };

            // If batching is enabled, deserialize batch and yield individual items
            if batching {
              match crate::batching::deserialize_batch(decompressed_bytes) {
                Ok(batch_items) => {
                  // Yield each item from the batch
                  for item_bytes in batch_items {
                    let deserializer = crate::serialization::ZeroCopyDeserializer::new(item_bytes);
                    match deserializer.deserialize::<<T as Input>::Input>() {
                      Ok(item) => yield item,
                      Err(e) => {
                        warn!(
                          error = %e,
                          "Failed to deserialize item from batch, skipping"
                        );
                        continue;
                      }
                    }
                  }
                }
                Err(e) => {
                  warn!(
                    error = %e,
                    "Failed to deserialize batch, skipping"
                  );
                  continue;
                }
              }
            } else {
              // Regular single item deserialization
              let deserializer = crate::serialization::ZeroCopyDeserializer::new(decompressed_bytes);
              match deserializer.deserialize::<<T as Input>::Input>() {
                Ok(item) => yield item,
                Err(e) => {
                  warn!(
                    error = %e,
                    "Failed to deserialize input item, skipping"
                  );
                  continue;
                }
              }
            }
          }
          ChannelItem::Arc(arc) => {
            // In-process mode: downcast Arc to T::Input
            // Note: This requires <T as Input>::Input: Send + Sync + 'static
            // The trait bounds on StreamWrapper ensure this
            match Arc::downcast::<<T as Input>::Input>(arc) {
              Ok(typed_arc) => {
                // Try to unwrap Arc for zero-cost extraction, otherwise clone
                let item = Arc::try_unwrap(typed_arc)
                  .unwrap_or_else(|arc| (*arc).clone());
                yield item;
              }
              Err(_) => {
                warn!(
                  "Failed to downcast Arc to input type, skipping"
                );
                continue;
              }
            }
          }
          ChannelItem::SharedMemory(_) => {
            // Shared memory mode - not yet implemented in StreamWrapper
            warn!("SharedMemory items not yet supported in StreamWrapper");
            continue;
          }
        }
      }
    });
    Self {
      stream,
      _phantom: PhantomData,
    }
  }

  /// Creates a new `StreamWrapper` from an empty stream.
  ///
  /// # Returns
  ///
  /// A new `StreamWrapper` containing an empty stream.
  fn empty() -> Self {
    Self {
      stream: Box::pin(futures::stream::empty::<<T as Input>::Input>()),
      _phantom: PhantomData,
    }
  }

  /// Creates a `StreamWrapper` by merging multiple input streams.
  ///
  /// This method takes multiple receivers and creates a merged stream that interleaves
  /// items from all input streams using `futures::stream::select_all` (fair interleaving).
  ///
  /// # Arguments
  ///
  /// * `receivers` - Iterator over `(port_name, receiver)` tuples from type-erased input channels
  /// * `compression` - Optional compression algorithm if compression is enabled
  /// * `batching` - Whether batching is enabled (batches need to be deserialized)
  ///
  /// # Returns
  ///
  /// A new `StreamWrapper` containing the merged stream from all inputs.
  fn from_multiple_receivers(
    receivers: impl Iterator<Item = (String, TypeErasedReceiver)>,
    compression: Option<CompressionAlgorithm>,
    batching: bool,
  ) -> Self {
    // Create streams from each receiver
    let streams: Vec<_> = receivers
      .map(|(_port_name, receiver)| {
        let comp = compression;
        Box::pin(stream! {
          let mut recv = receiver;
          while let Some(channel_item) = recv.recv().await {
            match channel_item {
              ChannelItem::Bytes(bytes) => {
                // Distributed mode: decompress if needed, then deserialize
                let decompressed_bytes = if let Some(algorithm) = comp {
                  let decompressor: Box<dyn Compression> = match algorithm {
                    CompressionAlgorithm::Gzip { level } => {
                      Box::new(crate::compression::GzipCompression::new(level))
                    }
                    CompressionAlgorithm::Zstd { level } => {
                      Box::new(crate::compression::ZstdCompression::new(level))
                    }
                  };

                  match tokio::task::spawn_blocking(move || decompressor.decompress(&bytes)).await {
                    Ok(Ok(decompressed)) => decompressed,
                    Ok(Err(e)) => {
                      error!(
                        error = %e,
                        "Failed to decompress input item from channel, skipping"
                      );
                      continue;
                    }
                    Err(e) => {
                      error!(
                        error = %e,
                        "Decompression task failed, skipping"
                      );
                      continue;
                    }
                  }
                } else {
                  bytes
                };

                // If batching is enabled, deserialize batch and yield individual items
                if batching {
                  match crate::batching::deserialize_batch(decompressed_bytes) {
                    Ok(batch_items) => {
                      // Yield each item from the batch
                      for item_bytes in batch_items {
                        let deserializer = crate::serialization::ZeroCopyDeserializer::new(item_bytes);
                        match deserializer.deserialize::<<T as Input>::Input>() {
                          Ok(item) => yield item,
                          Err(e) => {
                            error!(
                              error = %e,
                              "Failed to deserialize item from batch, skipping"
                            );
                            continue;
                          }
                        }
                      }
                    }
                    Err(e) => {
                      error!(
                        error = %e,
                        "Failed to deserialize batch, skipping"
                      );
                      continue;
                    }
                  }
                } else {
                  // Regular single item deserialization
                  let bytes_len = decompressed_bytes.len();
                  let deserializer = crate::serialization::ZeroCopyDeserializer::new(decompressed_bytes);
                  match deserializer.deserialize::<<T as Input>::Input>() {
                    Ok(item) => yield item,
                    Err(e) => {
                      error!(
                        error = %e,
                        bytes_len = bytes_len,
                        "Failed to deserialize input item from channel, skipping"
                      );
                      continue;
                    }
                  }
                }
              }
              ChannelItem::Arc(arc) => {
                // In-process mode: downcast Arc to T::Input
                match Arc::downcast::<<T as Input>::Input>(arc) {
                  Ok(typed_arc) => {
                    let item = Arc::try_unwrap(typed_arc)
                      .unwrap_or_else(|arc| (*arc).clone());
                    yield item;
                  }
                  Err(_) => {
                    error!(
                      "Failed to downcast Arc to input type, skipping"
                    );
                    continue;
                  }
                }
              }
              ChannelItem::SharedMemory(_) => {
                warn!("SharedMemory items not yet supported in StreamWrapper");
                continue;
              }
            }
          }
        }) as Pin<Box<dyn futures::Stream<Item = <T as Input>::Input> + Send>>
      })
      .collect();

    // Merge all streams using select_all (fair interleaving)
    // Box as trait object to match the expected type
    let merged_stream: Pin<Box<dyn futures::Stream<Item = <T as Input>::Input> + Send>> =
      if streams.is_empty() {
        Box::pin(futures::stream::empty::<<T as Input>::Input>())
      } else {
        Box::pin(futures::stream::select_all(streams))
      };

    Self {
      stream: merged_stream,
      _phantom: PhantomData,
    }
  }
}

/// Private trait for converting a stream wrapper to the associated InputStream type.
///
/// This trait encapsulates the unsafe conversion logic needed to convert
/// `Pin<Box<dyn Stream<...>>>` to `T::InputStream`. The conversion is safe because
/// all `InputStream` implementations in the codebase are `Pin<Box<dyn Stream<...>>>`.
trait StreamConverter<T: Input> {
  /// Converts the wrapped stream to `T::InputStream`.
  ///
  /// # Safety
  ///
  /// This method uses unsafe code to convert between types that are identical at runtime
  /// but different in Rust's type system. It is safe because:
  /// 1. All `InputStream` implementations are `Pin<Box<dyn Stream<Item = T::Input> + Send>>`
  /// 2. We've verified this across all transformer `Input` implementations
  /// 3. The memory layout is identical (both are trait objects)
  /// 4. The type parameter `T` ensures compile-time type safety
  fn into_input_stream(self) -> <T as Input>::InputStream;
}

impl<T: Input> StreamConverter<T> for StreamWrapper<T> {
  fn into_input_stream(self) -> <T as Input>::InputStream {
    // SAFETY: This conversion uses unsafe pointer manipulation because Rust's type system
    // doesn't allow direct conversion to associated types, even when they're identical.
    //
    // This is safe because:
    // 1. All InputStream implementations in the codebase are Pin<Box<dyn Stream<Item = T::Input> + Send>>
    // 2. We've verified this by examining all transformer Input implementations
    // 3. The memory layout is identical (both are Pin<Box<dyn Stream>> trait objects)
    // 4. The type parameter T ensures compile-time type safety through PhantomData
    //
    // We use ManuallyDrop to prevent double-drop, then read the value through a pointer cast.
    let stream = std::mem::ManuallyDrop::new(self.stream);
    unsafe {
      // Create a pointer to the stream, cast it to a pointer of the associated type,
      // then read the value. This is safe because both types have identical memory representation.
      std::ptr::read(
        &*stream as *const Pin<Box<dyn futures::Stream<Item = <T as Input>::Input> + Send>>
          as *const <T as Input>::InputStream,
      )
    }
  }
}

/// Helper function to transform a single item using ZeroCopyTransformer.
///
/// This function requires that the transformer implements ZeroCopyTransformer
/// and uses transform_zero_copy for zero-copy transformation.
#[allow(dead_code)]
fn transform_item_with_zero_copy<T>(transformer: &mut T, input: T::Input) -> T::Output
where
  T: crate::zero_copy::ZeroCopyTransformer,
  T::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  use std::borrow::Cow;

  // Use ZeroCopyTransformer::transform_zero_copy
  let cow_input = Cow::Owned(input);
  let cow_output = transformer.transform_zero_copy(cow_input);
  match cow_output {
    Cow::Borrowed(borrowed) => borrowed.clone(),
    Cow::Owned(owned) => owned,
  }
}

/// Helper function to process a stream using ZeroCopyTransformer when available.
///
/// This function processes the input stream item-by-item, using ZeroCopyTransformer
/// if the transformer implements it. For transformers that implement ZeroCopyTransformer,
/// items are wrapped in Cow::Owned and transformed using transform_zero_copy.
/// Otherwise, the regular transform method is used.
///
/// # Type Parameters
///
/// * `T` - The transformer type that implements `Transformer` and optionally `ZeroCopyTransformer`
///
/// # Arguments
///
/// * `transformer` - The transformer to use
/// * `input_stream` - The input stream to transform
///
/// # Returns
///
/// The transformed output stream
///
/// # Note
///
/// This function uses trait bounds to determine if ZeroCopyTransformer is available.
/// For transformers that implement ZeroCopyTransformer, it processes items one by one
/// using transform_zero_copy. For others, it falls back to the regular transform method.
#[allow(dead_code)]
async fn process_stream_with_zero_copy<T>(
  mut transformer: T,
  input_stream: T::InputStream,
) -> T::OutputStream
where
  T: Transformer,
  T::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  // Default implementation: use regular transform
  // For transformers that implement ZeroCopyTransformer, the specialized
  // version process_with_zero_copy_impl will be used via trait bounds.
  // However, since Rust doesn't support specialization in stable, we use
  // the regular transform method for now. The zero-copy optimization
  // is available via process_with_zero_copy_impl for transformers that
  // implement ZeroCopyTransformer, but requires explicit type bounds.
  transformer.transform(input_stream).await
}

/// Specialized helper for transformers that implement ZeroCopyTransformer.
///
/// This processes items one by one using transform_zero_copy for zero-copy semantics.
/// Items are wrapped in Cow::Owned and transformed, then extracted from the returned Cow.
#[allow(dead_code)]
async fn process_with_zero_copy_impl<T>(
  mut transformer: T,
  input_stream: T::InputStream,
) -> T::OutputStream
where
  T: Transformer + ZeroCopyTransformer + Send,
  T::Input: std::fmt::Debug + Clone + Send + Sync + 'static,
  T::Output: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  // Process stream item-by-item using zero-copy transformation
  let mut pinned_input = pin!(input_stream);
  let output_stream: Pin<Box<dyn futures::Stream<Item = T::Output> + Send>> =
    Box::pin(async_stream::stream! {
      while let Some(item) = pinned_input.next().await {
        // Wrap item in Cow::Owned and transform using zero-copy
        let cow_input = Cow::Owned(item);
        let cow_output = transformer.transform_zero_copy(cow_input);

        // Extract the output value from Cow
        let output = match cow_output {
          Cow::Borrowed(borrowed) => borrowed.clone(),
          Cow::Owned(owned) => owned,
        };

        yield output;
      }
    });

  // Convert to the expected output stream type using unsafe conversion
  // This is safe because all OutputStream implementations are Pin<Box<dyn Stream<...>>>
  let stream = std::mem::ManuallyDrop::new(output_stream);
  unsafe {
    std::ptr::read(
      &*stream as *const Pin<Box<dyn futures::Stream<Item = T::Output> + Send>>
        as *const T::OutputStream,
    )
  }
}

/// A node that wraps a Producer component.
///
/// ProducerNode represents a source node in the graph that generates data streams.
/// It has no input ports (Inputs = ()) and one or more output ports specified by
/// the `Outputs` type parameter.
///
/// # Architecture
///
/// The producer is stored internally as `Arc<tokio::sync::Mutex<P>>` to enable zero-copy
/// node sharing. This eliminates the `Clone` requirement for producers, allowing
/// control flow transformers and other non-cloneable components to be used in graphs.
///
/// # Type Parameters
///
/// * `P` - The producer type that implements `Producer` (does not need to implement `Clone`)
/// * `Outputs` - A port tuple representing the output ports (e.g., `(i32,)` for single port)
///
/// # Example
///
/// ```rust
/// use streamweave::graph::node::ProducerNode;
/// use streamweave_array::ArrayProducer;
///
/// let node = ProducerNode::new(
///     "source".to_string(),
///     ArrayProducer::new(vec![1, 2, 3]),
/// );
/// ```
pub struct ProducerNode<P, Outputs>
where
  P: Producer,
  P::Output: std::fmt::Debug + Clone + Send + Sync,
  Outputs: PortList,
  (): ValidateProducerPorts<P, Outputs>,
{
  name: String,
  producer: Arc<Mutex<P>>,
  output_port_names: Vec<String>,
  _phantom: std::marker::PhantomData<Outputs>,
  // Router deferred to Phase 2
}

impl<P, Outputs> ProducerNode<P, Outputs>
where
  P: Producer,
  P::Output: std::fmt::Debug + Clone + Send + Sync,
  Outputs: PortList,
  (): ValidateProducerPorts<P, Outputs>,
{
  /// Creates a new ProducerNode with the given name, producer, and output port names.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of this node
  /// * `producer` - The producer component to wrap
  /// * `output_port_names` - The names of the output ports (must match Outputs::LEN)
  ///
  /// # Returns
  ///
  /// A new `ProducerNode` instance.
  ///
  /// # Panics
  ///
  /// Panics if the number of port names doesn't match `Outputs::LEN`.
  pub fn new(name: String, producer: P, output_port_names: Vec<String>) -> Self {
    assert_eq!(
      output_port_names.len(),
      Outputs::LEN,
      "Number of output port names ({}) must match Outputs::LEN ({})",
      output_port_names.len(),
      Outputs::LEN
    );
    Self {
      name,
      producer: Arc::new(Mutex::new(producer)),
      output_port_names,
      _phantom: std::marker::PhantomData,
    }
  }

  /// Sets the name of this node.
  ///
  /// # Arguments
  ///
  /// * `name` - The new name for this node
  ///
  /// # Returns
  ///
  /// The node with the updated name.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.name = name;
    self
  }

  /// Returns the name of this node.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Returns a clone of the Arc containing the wrapped producer.
  ///
  /// This method returns `Arc<tokio::sync::Mutex<P>>` which can be cloned
  /// zero-copy and locked when needed. The producer does not need to implement `Clone`.
  ///
  /// # Returns
  ///
  /// A clone of the `Arc` containing the producer wrapped in a `Mutex`.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave_graph::ProducerNode;
  /// use streamweave_array::ArrayProducer;
  ///
  /// let node = ProducerNode::new("source".to_string(), ArrayProducer::new(vec![1, 2, 3]));
  /// let producer_arc = node.producer();
  /// // Lock and use the producer
  /// // let mut producer = producer_arc.lock().await;
  /// ```
  pub fn producer(&self) -> Arc<Mutex<P>> {
    Arc::clone(&self.producer)
  }
}

/// Type inference helper for ProducerNode using default port types.
///
/// This implementation allows creating ProducerNode instances without
/// explicitly specifying the Outputs type parameter. The port tuple is
/// automatically inferred from the producer's default output ports.
impl<P> ProducerNode<P, <P as ProducerPorts>::DefaultOutputPorts>
where
  P: Producer + ProducerPorts,
  P::Output: std::fmt::Debug + Clone + Send + Sync,
  (): ValidateProducerPorts<P, <P as ProducerPorts>::DefaultOutputPorts>,
{
  /// Creates a new ProducerNode with type inference from the producer's OutputPorts.
  ///
  /// This method automatically infers the output port tuple from the producer's
  /// `OutputPorts` associated type, eliminating the need for explicit type annotations.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of this node
  /// * `producer` - The producer component to wrap
  ///
  /// # Returns
  ///
  /// A new `ProducerNode` instance with inferred port types.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::graph::node::ProducerNode;
  /// use streamweave_vec::VecProducer;
  ///
  /// // Type inference: OutputPorts is automatically (i32,)
  /// let node = ProducerNode::from_producer(
  ///     "source".to_string(),
  ///     VecProducer::new(vec![1, 2, 3])
  /// );
  /// ```
  pub fn from_producer(name: String, producer: P) -> Self {
    // Default port names: single port gets "out", multiple ports get "out_0", "out_1", etc.
    let output_port_names = if <P as ProducerPorts>::DefaultOutputPorts::LEN == 1 {
      vec!["out".to_string()]
    } else {
      (0..<P as ProducerPorts>::DefaultOutputPorts::LEN)
        .map(|i| format!("out_{}", i))
        .collect()
    };
    Self {
      name,
      producer: Arc::new(Mutex::new(producer)),
      output_port_names,
      _phantom: std::marker::PhantomData,
    }
  }

  /// Creates a new ProducerNode from a producer, using the producer's name if available.
  ///
  /// This is a convenience method that extracts the name from the producer's config
  /// if available, otherwise uses a default name.
  ///
  /// # Arguments
  ///
  /// * `producer` - The producer component to wrap
  ///
  /// # Returns
  ///
  /// A new `ProducerNode` instance with inferred port types.
  pub fn from(producer: P) -> Self
  where
    P: Producer,
  {
    let name = producer
      .config()
      .name()
      .clone()
      .unwrap_or_else(|| "producer".to_string());
    // Default port names: single port gets "out", multiple ports get "out_0", "out_1", etc.
    let output_port_names = if <P as ProducerPorts>::DefaultOutputPorts::LEN == 1 {
      vec!["out".to_string()]
    } else {
      (0..<P as ProducerPorts>::DefaultOutputPorts::LEN)
        .map(|i| format!("out_{}", i))
        .collect()
    };
    Self {
      name,
      producer: Arc::new(Mutex::new(producer)),
      output_port_names,
      _phantom: std::marker::PhantomData,
    }
  }
}

/// A node that wraps a Transformer component.
///
/// TransformerNode represents a processing node in the graph that transforms data streams.
/// It has one or more input ports and one or more output ports specified by the
/// `Inputs` and `Outputs` type parameters.
///
/// # Type Parameters
///
/// * `T` - The transformer type that implements `Transformer`
/// * `Inputs` - A port tuple representing the input ports (e.g., `(i32,)` for single port)
/// * `Outputs` - A port tuple representing the output ports (e.g., `(String,)` for single port)
///
/// # Example
///
/// ```rust
/// use streamweave::graph::node::TransformerNode;
/// use streamweave_transformers::MapTransformer;
///
/// let node = TransformerNode::new(
///     "mapper".to_string(),
///     MapTransformer::new(|x: i32| x * 2),
/// );
/// ```
pub struct TransformerNode<T, Inputs, Outputs>
where
  T: Transformer,
  T::Input: std::fmt::Debug + Clone + Send + Sync,
  T::Output: std::fmt::Debug + Clone + Send + Sync,
  Inputs: PortList,
  Outputs: PortList,
  (): ValidateTransformerPorts<T, Inputs, Outputs>,
{
  name: String,
  transformer: Arc<Mutex<T>>,
  input_port_names: Vec<String>,
  output_port_names: Vec<String>,
  _phantom: std::marker::PhantomData<(Inputs, Outputs)>,
  // Routers deferred to Phase 2
}

impl<T, Inputs, Outputs> TransformerNode<T, Inputs, Outputs>
where
  T: Transformer,
  T::Input: std::fmt::Debug + Clone + Send + Sync,
  T::Output: std::fmt::Debug + Clone + Send + Sync,
  Inputs: PortList,
  Outputs: PortList,
  (): ValidateTransformerPorts<T, Inputs, Outputs>,
{
  /// Creates a new TransformerNode with the given name, transformer, and port names.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of this node
  /// * `transformer` - The transformer component to wrap
  /// * `input_port_names` - The names of the input ports (must match Inputs::LEN)
  /// * `output_port_names` - The names of the output ports (must match Outputs::LEN)
  ///
  /// # Returns
  ///
  /// A new `TransformerNode` instance.
  ///
  /// # Panics
  ///
  /// Panics if the number of port names doesn't match `Inputs::LEN` or `Outputs::LEN`.
  pub fn new(
    name: String,
    transformer: T,
    input_port_names: Vec<String>,
    output_port_names: Vec<String>,
  ) -> Self {
    assert_eq!(
      input_port_names.len(),
      Inputs::LEN,
      "Number of input port names ({}) must match Inputs::LEN ({})",
      input_port_names.len(),
      Inputs::LEN
    );
    assert_eq!(
      output_port_names.len(),
      Outputs::LEN,
      "Number of output port names ({}) must match Outputs::LEN ({})",
      output_port_names.len(),
      Outputs::LEN
    );
    Self {
      name,
      transformer: Arc::new(Mutex::new(transformer)),
      input_port_names,
      output_port_names,
      _phantom: std::marker::PhantomData,
    }
  }

  /// Sets the name of this node.
  ///
  /// # Arguments
  ///
  /// * `name` - The new name for this node
  ///
  /// # Returns
  ///
  /// The node with the updated name.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.name = name;
    self
  }

  /// Returns the name of this node.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Returns a clone of the Arc containing the wrapped transformer.
  ///
  /// This method returns `Arc<tokio::sync::Mutex<T>>` which can be cloned
  /// zero-copy and locked when needed. The transformer does not need to implement `Clone`.
  ///
  /// # Returns
  ///
  /// A clone of the `Arc` containing the transformer wrapped in a `Mutex`.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave_graph::TransformerNode;
  /// use streamweave_transformers::MapTransformer;
  ///
  /// let node = TransformerNode::new("mapper".to_string(), MapTransformer::new(|x: i32| x * 2));
  /// let transformer_arc = node.transformer();
  /// // Lock and use the transformer
  /// // let mut transformer = transformer_arc.lock().await;
  /// ```
  pub fn transformer(&self) -> Arc<Mutex<T>> {
    Arc::clone(&self.transformer)
  }
}

/// Type inference helper for TransformerNode using default port types.
///
/// This implementation allows creating TransformerNode instances without
/// explicitly specifying the Inputs and Outputs type parameters. The port tuples
/// are automatically inferred from the transformer's default port types.
impl<T>
  TransformerNode<
    T,
    <T as TransformerPorts>::DefaultInputPorts,
    <T as TransformerPorts>::DefaultOutputPorts,
  >
where
  T: Transformer + TransformerPorts,
  T::Input: std::fmt::Debug + Clone + Send + Sync,
  T::Output: std::fmt::Debug + Clone + Send + Sync,
  (): ValidateTransformerPorts<
      T,
      <T as TransformerPorts>::DefaultInputPorts,
      <T as TransformerPorts>::DefaultOutputPorts,
    >,
{
  /// Creates a new TransformerNode with type inference from the transformer's port types.
  ///
  /// This method automatically infers the input and output port tuples from the transformer's
  /// `InputPorts` and `OutputPorts` associated types, eliminating the need for explicit
  /// type annotations.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of this node
  /// * `transformer` - The transformer component to wrap
  ///
  /// # Returns
  ///
  /// A new `TransformerNode` instance with inferred port types.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::graph::node::TransformerNode;
  /// use streamweave_transformers::MapTransformer;
  ///
  /// // Type inference: InputPorts is (i32,), OutputPorts is (i32,)
  /// let node = TransformerNode::from_transformer(
  ///     "mapper".to_string(),
  ///     MapTransformer::new(|x: i32| x * 2)
  /// );
  /// ```
  pub fn from_transformer(name: String, transformer: T) -> Self {
    // Default port names
    let input_port_names = if <T as TransformerPorts>::DefaultInputPorts::LEN == 1 {
      vec!["in".to_string()]
    } else {
      (0..<T as TransformerPorts>::DefaultInputPorts::LEN)
        .map(|i| format!("in_{}", i))
        .collect()
    };
    let output_port_names = if <T as TransformerPorts>::DefaultOutputPorts::LEN == 1 {
      vec!["out".to_string()]
    } else {
      (0..<T as TransformerPorts>::DefaultOutputPorts::LEN)
        .map(|i| format!("out_{}", i))
        .collect()
    };
    Self {
      name,
      transformer: Arc::new(Mutex::new(transformer)),
      input_port_names,
      output_port_names,
      _phantom: std::marker::PhantomData,
    }
  }

  /// Creates a new TransformerNode from a transformer, using the transformer's name if available.
  ///
  /// This is a convenience method that extracts the name from the transformer's config
  /// if available, otherwise uses a default name.
  ///
  /// # Arguments
  ///
  /// * `transformer` - The transformer component to wrap
  ///
  /// # Returns
  ///
  /// A new `TransformerNode` instance with inferred port types.
  pub fn from(transformer: T) -> Self
  where
    T: Transformer,
  {
    let name = transformer
      .config()
      .name()
      .clone()
      .unwrap_or_else(|| "transformer".to_string());
    // Default port names
    let input_port_names = if <T as TransformerPorts>::DefaultInputPorts::LEN == 1 {
      vec!["in".to_string()]
    } else {
      (0..<T as TransformerPorts>::DefaultInputPorts::LEN)
        .map(|i| format!("in_{}", i))
        .collect()
    };
    let output_port_names = if <T as TransformerPorts>::DefaultOutputPorts::LEN == 1 {
      vec!["out".to_string()]
    } else {
      (0..<T as TransformerPorts>::DefaultOutputPorts::LEN)
        .map(|i| format!("out_{}", i))
        .collect()
    };
    Self {
      name,
      transformer: Arc::new(Mutex::new(transformer)),
      input_port_names,
      output_port_names,
      _phantom: std::marker::PhantomData,
    }
  }
}

/// A node that wraps a Consumer component.
///
/// ConsumerNode represents a sink node in the graph that consumes data streams.
/// It has one or more input ports specified by the `Inputs` type parameter and
/// no output ports (Outputs = ()).
///
/// # Architecture
///
/// The consumer is stored internally as `Arc<tokio::sync::Mutex<C>>` to enable zero-copy
/// node sharing. This eliminates the `Clone` requirement for consumers, allowing
/// control flow transformers and other non-cloneable components to be used in graphs.
///
/// # Type Parameters
///
/// * `C` - The consumer type that implements `Consumer` (does not need to implement `Clone`)
/// * `Inputs` - A port tuple representing the input ports (e.g., `(String,)` for single port)
///
/// # Example
///
/// ```rust
/// use streamweave::graph::node::ConsumerNode;
/// use streamweave_vec::VecConsumer;
///
/// let node = ConsumerNode::new(
///     "sink".to_string(),
///     VecConsumer::new(),
/// );
/// ```
pub struct ConsumerNode<C, Inputs>
where
  C: Consumer,
  C::Input: std::fmt::Debug + Clone + Send + Sync,
  Inputs: PortList,
  (): ValidateConsumerPorts<C, Inputs>,
{
  name: String,
  consumer: Arc<Mutex<C>>,
  input_port_names: Vec<String>,
  _phantom: std::marker::PhantomData<Inputs>,
  // Router deferred to Phase 2
}

impl<C, Inputs> ConsumerNode<C, Inputs>
where
  C: Consumer,
  C::Input: std::fmt::Debug + Clone + Send + Sync,
  Inputs: PortList,
  (): ValidateConsumerPorts<C, Inputs>,
{
  /// Creates a new ConsumerNode with the given name, consumer, and input port names.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of this node
  /// * `consumer` - The consumer component to wrap
  /// * `input_port_names` - The names of the input ports (must match Inputs::LEN)
  ///
  /// # Returns
  ///
  /// A new `ConsumerNode` instance.
  ///
  /// # Panics
  ///
  /// Panics if the number of port names doesn't match `Inputs::LEN`.
  pub fn new(name: String, consumer: C, input_port_names: Vec<String>) -> Self {
    assert_eq!(
      input_port_names.len(),
      Inputs::LEN,
      "Number of input port names ({}) must match Inputs::LEN ({})",
      input_port_names.len(),
      Inputs::LEN
    );
    Self {
      name,
      consumer: Arc::new(Mutex::new(consumer)),
      input_port_names,
      _phantom: std::marker::PhantomData,
    }
  }

  /// Sets the name of this node.
  ///
  /// # Arguments
  ///
  /// * `name` - The new name for this node
  ///
  /// # Returns
  ///
  /// The node with the updated name.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.name = name;
    self
  }

  /// Returns the name of this node.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Returns a clone of the Arc containing the wrapped consumer.
  ///
  /// This method returns `Arc<tokio::sync::Mutex<C>>` which can be cloned
  /// zero-copy and locked when needed. The consumer does not need to implement `Clone`.
  ///
  /// # Returns
  ///
  /// A clone of the `Arc` containing the consumer wrapped in a `Mutex`.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave_graph::ConsumerNode;
  /// use streamweave_vec::VecConsumer;
  ///
  /// let node = ConsumerNode::new("sink".to_string(), VecConsumer::<i32>::new());
  /// let consumer_arc = node.consumer();
  /// // Lock and use the consumer
  /// // let mut consumer = consumer_arc.lock().await;
  /// ```
  pub fn consumer(&self) -> Arc<Mutex<C>> {
    Arc::clone(&self.consumer)
  }
}

/// Type inference helper for ConsumerNode using default port types.
///
/// This implementation allows creating ConsumerNode instances without
/// explicitly specifying the Inputs type parameter. The port tuple is
/// automatically inferred from the consumer's default input ports.
impl<C> ConsumerNode<C, <C as ConsumerPorts>::DefaultInputPorts>
where
  C: Consumer + ConsumerPorts,
  C::Input: std::fmt::Debug + Clone + Send + Sync,
  (): ValidateConsumerPorts<C, <C as ConsumerPorts>::DefaultInputPorts>,
{
  /// Creates a new ConsumerNode with type inference from the consumer's InputPorts.
  ///
  /// This method automatically infers the input port tuple from the consumer's
  /// `InputPorts` associated type, eliminating the need for explicit type annotations.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of this node
  /// * `consumer` - The consumer component to wrap
  ///
  /// # Returns
  ///
  /// A new `ConsumerNode` instance with inferred port types.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::graph::node::ConsumerNode;
  /// use streamweave_vec::VecConsumer;
  ///
  /// // Type inference: InputPorts is automatically (i32,)
  /// let node = ConsumerNode::from_consumer(
  ///     "sink".to_string(),
  ///     VecConsumer::new()
  /// );
  /// ```
  pub fn from_consumer(name: String, consumer: C) -> Self {
    // Default port names: single port gets "in", multiple ports get "in_0", "in_1", etc.
    let input_port_names = if <C as ConsumerPorts>::DefaultInputPorts::LEN == 1 {
      vec!["in".to_string()]
    } else {
      (0..<C as ConsumerPorts>::DefaultInputPorts::LEN)
        .map(|i| format!("in_{}", i))
        .collect()
    };
    Self {
      name,
      consumer: Arc::new(Mutex::new(consumer)),
      input_port_names,
      _phantom: std::marker::PhantomData,
    }
  }

  /// Creates a new ConsumerNode from a consumer, using the consumer's name if available.
  ///
  /// This is a convenience method that extracts the name from the consumer's config
  /// if available, otherwise uses a default name.
  ///
  /// # Arguments
  ///
  /// * `consumer` - The consumer component to wrap
  ///
  /// # Returns
  ///
  /// A new `ConsumerNode` instance with inferred port types.
  pub fn from(consumer: C) -> Self
  where
    C: Consumer,
  {
    let name = consumer.config().name.clone();
    // Default port names: single port gets "in", multiple ports get "in_0", "in_1", etc.
    let input_port_names = if <C as ConsumerPorts>::DefaultInputPorts::LEN == 1 {
      vec!["in".to_string()]
    } else {
      (0..<C as ConsumerPorts>::DefaultInputPorts::LEN)
        .map(|i| format!("in_{}", i))
        .collect()
    };
    Self {
      name,
      consumer: Arc::new(Mutex::new(consumer)),
      input_port_names,
      _phantom: std::marker::PhantomData,
    }
  }
}

// Implement NodeTrait for ProducerNode
impl<P, Outputs> NodeTrait for ProducerNode<P, Outputs>
where
  P: Producer + Send + Sync + 'static,
  P::Output: std::fmt::Debug + Clone + Send + Sync + Serialize,
  Outputs: PortList + Send + Sync + 'static,
  (): ValidateProducerPorts<P, Outputs>,
{
  fn name(&self) -> &str {
    &self.name
  }

  fn node_kind(&self) -> NodeKind {
    NodeKind::Producer
  }

  fn input_port_names(&self) -> Vec<String> {
    vec![] // Producers have no input ports
  }

  fn output_port_names(&self) -> Vec<String> {
    self.output_port_names.clone()
  }

  fn has_input_port(&self, _port_name: &str) -> bool {
    false // Producers have no input ports
  }

  fn has_output_port(&self, port_name: &str) -> bool {
    self.output_port_names.iter().any(|name| name == port_name)
  }

  fn spawn_execution_task(
    &self,
    _input_channels: std::collections::HashMap<String, TypeErasedReceiver>,
    output_channels: std::collections::HashMap<String, TypeErasedSender>,
    pause_signal: std::sync::Arc<tokio::sync::RwLock<bool>>,
    execution_mode: crate::execution::ExecutionMode,
    batching_channels: Option<
      std::collections::HashMap<String, std::sync::Arc<crate::batching::BatchingChannel>>,
    >,
    arc_pool: Option<std::sync::Arc<crate::zero_copy::ArcPool<bytes::Bytes>>>,
  ) -> Option<tokio::task::JoinHandle<Result<(), crate::execution::ExecutionError>>> {
    // Implementation requires P::Output: Serialize (added to trait bounds above)
    // This implementation:
    // 1. Clones the Arc containing the producer (zero-copy)
    // 2. Locks the mutex and calls producer.produce() to get the stream
    // 3. Serializes each item to Bytes
    // 4. Sends to output channels (broadcasts to all ports)
    // 5. Handles pause signal and errors
    //
    // NOTE: Task 13.2.2 - In-process zero-copy mode will change output_channels type
    // from mpsc::Sender<Bytes> to mpsc::Sender<Arc<P::Output>> when ExecutionMode::InProcess
    // is implemented (task 15). This will eliminate serialization overhead for in-process execution.

    let node_name = self.name.clone();
    let producer = Arc::clone(&self.producer);
    let _batching_channels_clone = batching_channels.clone();
    let arc_pool_clone = arc_pool.clone();

    let handle = tokio::spawn(async move {
      // Get the stream from the producer and pin it
      // Note: produce() takes &self, so we call it with a lock then drop the guard
      // The stream should own what it needs, so we release the lock
      let stream = {
        let mut producer_guard = producer.lock().await;
        producer_guard.produce()
      };
      let mut stream = pin!(stream);

      // Iterate over stream items
      loop {
        // Use timeout to periodically check for shutdown
        let item_result =
          tokio::time::timeout(tokio::time::Duration::from_millis(100), stream.next()).await;

        let item = match item_result {
          Ok(Some(item)) => item,
          Ok(None) => break, // Stream exhausted
          Err(_) => {
            // Timeout - check if we should exit (shutdown)
            let paused = *pause_signal.read().await;
            if paused {
              // Pause signal set - during shutdown, exit gracefully
              return Ok(());
            }
            // Not paused, continue waiting for stream item
            continue;
          }
        };

        // Check pause signal before processing each item
        // If paused, wait briefly with timeout to avoid blocking during shutdown
        // During shutdown, the timeout check above will detect pause and exit
        let pause_check_result =
          tokio::time::timeout(tokio::time::Duration::from_millis(100), async {
            while *pause_signal.read().await {
              tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
          })
          .await;

        // If timeout occurred and we're still paused, might be shutdown
        // Exit immediately to avoid blocking during shutdown
        if pause_check_result.is_err() && *pause_signal.read().await {
          // Still paused after timeout - likely shutdown, exit gracefully
          return Ok(());
        }

        // Check execution mode for zero-copy optimization
        let is_in_process = matches!(
          execution_mode,
          crate::execution::ExecutionMode::InProcess { .. }
        );
        let is_fan_out = output_channels.len() > 1;

        // In in-process mode: use Arc<T> for true zero-copy (no serialization)
        // In distributed mode: serialize to Bytes
        let mut send_succeeded = 0;
        let mut send_failed_count = 0;

        if is_in_process {
          // In-process zero-copy mode: wrap item in Arc and send directly (no serialization)
          let item_arc = Arc::new(item);

          if is_fan_out {
            // Fan-out: clone Arc (zero-copy) to all outputs
            for (port_name, sender) in &output_channels {
              // Convert Arc<T> to Arc<dyn Any + Send + Sync> for type erasure
              let arc_any: Arc<dyn Any + Send + Sync> = unsafe {
                Arc::from_raw(Arc::into_raw(item_arc.clone()) as *const (dyn Any + Send + Sync))
              };
              match sender.send(ChannelItem::Arc(arc_any)).await {
                Ok(()) => {
                  send_succeeded += 1;
                }
                Err(_e) => {
                  let paused = *pause_signal.read().await;
                  if paused {
                    return Ok(());
                  }
                  send_failed_count += 1;
                  warn!(
                    node = %node_name,
                    port = %port_name,
                    "Output channel receiver dropped (may be normal in fan-out scenarios)"
                  );
                }
              }
            }
          } else {
            // Single output: send Arc directly
            let (port_name, sender) = output_channels.iter().next().unwrap();
            // Convert Arc<T> to Arc<dyn Any + Send + Sync> for type erasure
            let arc_any: Arc<dyn Any + Send + Sync> =
              unsafe { Arc::from_raw(Arc::into_raw(item_arc) as *const (dyn Any + Send + Sync)) };
            match sender.send(ChannelItem::Arc(arc_any)).await {
              Ok(()) => {
                send_succeeded += 1;
              }
              Err(_e) => {
                let paused = *pause_signal.read().await;
                if paused {
                  return Ok(());
                }
                send_failed_count += 1;
                warn!(
                  node = %node_name,
                  port = port_name,
                  "Output channel receiver dropped"
                );
              }
            }
          }
        } else {
          // Distributed mode: serialize to Bytes, then compress if enabled
          let serialized = match serialize(&item) {
            Ok(bytes) => bytes,
            Err(e) => {
              error!(
                node = %node_name,
                error = %e,
                "Failed to serialize producer output"
              );
              return Err(crate::execution::ExecutionError::SerializationError {
                node: node_name,
                is_deserialization: false,
                reason: format!("Failed to serialize producer output: {}", e),
              });
            }
          };

          // Apply compression if configured
          let final_bytes = if let crate::execution::ExecutionMode::Distributed {
            compression: Some(algorithm),
            ..
          } = &execution_mode
          {
            // Create compression instance with configured level
            let compressor: Box<dyn Compression> = match *algorithm {
              CompressionAlgorithm::Gzip { level } => {
                Box::new(crate::compression::GzipCompression::new(level))
              }
              CompressionAlgorithm::Zstd { level } => {
                Box::new(crate::compression::ZstdCompression::new(level))
              }
            };

            // Compress in blocking task (compression is CPU-intensive)
            match tokio::task::spawn_blocking(move || compressor.compress(&serialized)).await {
              Ok(Ok(compressed)) => compressed,
              Ok(Err(e)) => {
                error!(
                  node = %node_name,
                  error = %e,
                  "Failed to compress producer output"
                );
                let is_corrupted = matches!(e, crate::compression::CompressionError::CorruptedData);
                return Err(crate::execution::ExecutionError::CompressionError {
                  node: node_name,
                  is_compression: true,
                  reason: e.to_string(),
                  is_corrupted,
                });
              }
              Err(e) => {
                error!(
                  node = %node_name,
                  error = %e,
                  "Compression task failed"
                );
                return Err(crate::execution::ExecutionError::CompressionError {
                  node: node_name,
                  is_compression: true,
                  reason: format!("Compression task failed: {}", e),
                  is_corrupted: false,
                });
              }
            }
          } else {
            serialized
          };

          if is_fan_out {
            // Fan-out: wrap in Arc for zero-copy sharing of serialized data
            // Use pool if available to reduce allocations
            let shared_bytes: Arc<bytes::Bytes> = if let Some(ref pool) = arc_pool_clone {
              pool.get_or_create(final_bytes)
            } else {
              Arc::new(final_bytes)
            };
            for (port_name, sender) in &output_channels {
              match sender
                .send(ChannelItem::Bytes(shared_bytes.as_ref().clone()))
                .await
              {
                Ok(()) => {
                  send_succeeded += 1;
                }
                Err(_e) => {
                  let paused = *pause_signal.read().await;
                  if paused {
                    return Ok(());
                  }
                  send_failed_count += 1;
                  warn!(
                    node = %node_name,
                    port = %port_name,
                    "Output channel receiver dropped (may be normal in fan-out scenarios)"
                  );
                }
              }
            }
          } else {
            // Single output: send Bytes directly
            let (port_name, sender) = output_channels.iter().next().unwrap();

            // Use batching channel if available, otherwise use regular sender
            let send_result = if let Some(batching_channel) =
              batching_channels.as_ref().and_then(|bc| bc.get(port_name))
            {
              batching_channel.send(ChannelItem::Bytes(final_bytes)).await
            } else {
              sender
                .send(ChannelItem::Bytes(final_bytes))
                .await
                .map_err(|e| crate::execution::ExecutionError::ChannelError {
                  node: node_name.clone(),
                  port: port_name.clone(),
                  is_input: false,
                  reason: format!("Failed to send item: {}", e),
                })
            };

            match send_result {
              Ok(()) => {
                send_succeeded += 1;
              }
              Err(_e) => {
                let paused = *pause_signal.read().await;
                if paused {
                  return Ok(());
                }
                send_failed_count += 1;
                warn!(
                  node = %node_name,
                  port = %port_name,
                  "Output channel receiver dropped"
                );
              }
            }
          }
        }

        // If ALL channels failed (and not shutdown), all downstream nodes have finished
        // Exit gracefully - this is normal behavior when downstream nodes complete
        if send_failed_count > 0 && send_succeeded == 0 {
          // All downstream nodes finished - exit gracefully
          return Ok(());
        }
      }

      Ok(())
    });

    Some(handle)
  }
}

// Implement NodeTrait for TransformerNode
impl<T, Inputs, Outputs> NodeTrait for TransformerNode<T, Inputs, Outputs>
where
  T: Transformer + Send + Sync + 'static,
  T::Input: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned,
  T::Output: std::fmt::Debug + Clone + Send + Sync + Serialize,
  Inputs: PortList + Send + Sync + 'static,
  Outputs: PortList + Send + Sync + 'static,
  (): ValidateTransformerPorts<T, Inputs, Outputs>,
{
  fn name(&self) -> &str {
    &self.name
  }

  fn node_kind(&self) -> NodeKind {
    NodeKind::Transformer
  }

  fn input_port_names(&self) -> Vec<String> {
    self.input_port_names.clone()
  }

  fn output_port_names(&self) -> Vec<String> {
    self.output_port_names.clone()
  }

  fn has_input_port(&self, port_name: &str) -> bool {
    self.input_port_names.iter().any(|name| name == port_name)
  }

  fn has_output_port(&self, port_name: &str) -> bool {
    self.output_port_names.iter().any(|name| name == port_name)
  }

  fn spawn_execution_task(
    &self,
    input_channels: std::collections::HashMap<String, TypeErasedReceiver>,
    output_channels: std::collections::HashMap<String, TypeErasedSender>,
    pause_signal: std::sync::Arc<tokio::sync::RwLock<bool>>,
    execution_mode: crate::execution::ExecutionMode,
    batching_channels: Option<
      std::collections::HashMap<String, std::sync::Arc<crate::batching::BatchingChannel>>,
    >,
    arc_pool: Option<std::sync::Arc<crate::zero_copy::ArcPool<bytes::Bytes>>>,
  ) -> Option<tokio::task::JoinHandle<Result<(), crate::execution::ExecutionError>>> {
    // NOTE: Serialization is handled via the serialize() function from the
    // serialization module. When ExecutionMode::Distributed is used, nodes
    // serialize data using this function. Future optimization: pass Serializer
    // trait directly to nodes for pluggable serialization formats.
    let node_name = self.name.clone();
    let transformer = Arc::clone(&self.transformer);
    let execution_mode_clone = execution_mode.clone();
    let _batching_channels_clone = batching_channels.clone();
    let arc_pool_clone = arc_pool.clone();

    let handle = tokio::spawn(async move {
      // Create input stream from channels by deserializing
      // Support both single and multiple input channels (fan-in)

      // Extract compression and batching info from execution mode
      let (compression, batching) = match &execution_mode {
        crate::execution::ExecutionMode::Distributed {
          compression: comp,
          batching: batch,
          ..
        } => (*comp, batch.is_some()),
        _ => (None, false),
      };

      // Create StreamWrapper using Option 6 (Phantom Type Pattern with Conversion Trait)
      let stream_wrapper: StreamWrapper<T> = if input_channels.is_empty() {
        StreamWrapper::empty()
      } else if input_channels.len() == 1 {
        // Single input: create stream from single receiver
        let (_port_name, receiver) = input_channels.into_iter().next().unwrap();
        StreamWrapper::from_receiver(receiver, compression, batching)
      } else {
        // Multiple inputs: merge streams from all receivers
        StreamWrapper::from_multiple_receivers(input_channels.into_iter(), compression, batching)
      };

      // Convert to T::InputStream using the StreamConverter trait
      // The unsafe conversion is encapsulated in the trait implementation
      let input_stream = stream_wrapper.into_input_stream();
      let mut input_stream = pin!(input_stream);

      // Process items one by one, using ZeroCopyTransformer if available
      // This enables zero-copy transformations when the transformer implements ZeroCopyTransformer
      loop {
        // Use timeout to periodically check for shutdown
        let item_result =
          tokio::time::timeout(tokio::time::Duration::from_millis(100), input_stream.next()).await;

        let input_item = match item_result {
          Ok(Some(item)) => item,
          Ok(None) => break, // Stream exhausted
          Err(_) => {
            // Timeout - check if we should exit (shutdown)
            let paused = *pause_signal.read().await;
            if paused {
              // Pause signal set - during shutdown, exit gracefully
              return Ok(());
            }
            // Not paused, continue waiting for stream item
            continue;
          }
        };

        // Check pause signal before processing each item
        let pause_check_result =
          tokio::time::timeout(tokio::time::Duration::from_millis(100), async {
            while *pause_signal.read().await {
              tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
          })
          .await;

        if pause_check_result.is_err() && *pause_signal.read().await {
          return Ok(());
        }

        // Transform item using the regular transform method
        // Create a single-item stream and transform it
        use futures::StreamExt;
        use futures::stream;
        let single_item_stream: Pin<Box<dyn futures::Stream<Item = T::Input> + Send>> =
          Box::pin(stream::once(async move { input_item }));
        // Convert to T::InputStream using unsafe conversion
        // This is safe because all InputStream implementations are Pin<Box<dyn Stream<...>>>
        let stream = std::mem::ManuallyDrop::new(single_item_stream);
        let input_stream: T::InputStream = unsafe {
          std::ptr::read(
            &*stream as *const Pin<Box<dyn futures::Stream<Item = T::Input> + Send>>
              as *const T::InputStream,
          )
        };
        let output_stream = {
          let mut transformer_guard = transformer.lock().await;
          transformer_guard.transform(input_stream).await
        };
        let mut output_stream = pin!(output_stream);
        let item = match output_stream.next().await {
          Some(item) => item,
          None => {
            // No output from transform - this shouldn't happen for a single item
            warn!("Transformer produced no output for input item, skipping");
            continue;
          }
        };

        // Check pause signal before processing each item
        // If paused, wait briefly with timeout to avoid blocking during shutdown
        // During shutdown, the timeout check above will detect pause and exit
        let pause_check_result =
          tokio::time::timeout(tokio::time::Duration::from_millis(100), async {
            while *pause_signal.read().await {
              tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
          })
          .await;

        // If timeout occurred and we're still paused, might be shutdown
        // Exit immediately to avoid blocking during shutdown
        if pause_check_result.is_err() && *pause_signal.read().await {
          // Still paused after timeout - likely shutdown, exit gracefully
          return Ok(());
        }

        // Check execution mode for zero-copy optimization
        let is_in_process = matches!(
          execution_mode_clone,
          crate::execution::ExecutionMode::InProcess { .. }
        );
        let is_fan_out = output_channels.len() > 1;

        // In in-process mode: use Arc<T> for true zero-copy (no serialization)
        // In distributed mode: serialize to Bytes
        let mut send_succeeded = 0;
        let mut send_failed_count = 0;

        if is_in_process {
          // In-process zero-copy mode: wrap item in Arc and send directly (no serialization)
          let item_arc = Arc::new(item);

          if is_fan_out {
            // Fan-out: clone Arc (zero-copy) to all outputs
            for (port_name, sender) in &output_channels {
              // Convert Arc<T> to Arc<dyn Any + Send + Sync> for type erasure
              let arc_any: Arc<dyn Any + Send + Sync> = unsafe {
                Arc::from_raw(Arc::into_raw(item_arc.clone()) as *const (dyn Any + Send + Sync))
              };
              match sender.send(ChannelItem::Arc(arc_any)).await {
                Ok(()) => {
                  send_succeeded += 1;
                }
                Err(_e) => {
                  let paused = *pause_signal.read().await;
                  if paused {
                    return Ok(());
                  }
                  send_failed_count += 1;
                  warn!(
                    node = %node_name,
                    port = %port_name,
                    "Output channel receiver dropped (may be normal in fan-out scenarios)"
                  );
                }
              }
            }
          } else {
            // Single output: send Arc directly
            let (_port_name, sender) = output_channels.iter().next().unwrap();
            // Convert Arc<T> to Arc<dyn Any + Send + Sync> for type erasure
            let arc_any: Arc<dyn Any + Send + Sync> =
              unsafe { Arc::from_raw(Arc::into_raw(item_arc) as *const (dyn Any + Send + Sync)) };
            match sender.send(ChannelItem::Arc(arc_any)).await {
              Ok(()) => {
                send_succeeded += 1;
              }
              Err(_e) => {
                let paused = *pause_signal.read().await;
                if paused {
                  return Ok(());
                }
                send_failed_count += 1;
                warn!(
                  node = %node_name,
                  port = %_port_name,
                  "Output channel receiver dropped"
                );
              }
            }
          }
        } else {
          // Distributed mode: serialize to Bytes, then compress if enabled
          let serialized = match serialize(&item) {
            Ok(bytes) => bytes,
            Err(e) => {
              error!(
                node = %node_name,
                error = %e,
                "Failed to serialize transformer output"
              );
              return Err(crate::execution::ExecutionError::SerializationError {
                node: node_name,
                is_deserialization: false,
                reason: format!("Failed to serialize transformer output: {}", e),
              });
            }
          };

          // Apply compression if configured
          let final_bytes = if let crate::execution::ExecutionMode::Distributed {
            compression: Some(algorithm),
            ..
          } = &execution_mode_clone
          {
            // Create compression instance with the configured level
            let compressor: Box<dyn Compression> = match *algorithm {
              CompressionAlgorithm::Gzip { level } => {
                Box::new(crate::compression::GzipCompression::new(level))
              }
              CompressionAlgorithm::Zstd { level } => {
                Box::new(crate::compression::ZstdCompression::new(level))
              }
            };

            // Compress in blocking task (compression is CPU-intensive)
            match tokio::task::spawn_blocking(move || compressor.compress(&serialized)).await {
              Ok(Ok(compressed)) => compressed,
              Ok(Err(e)) => {
                error!(
                  node = %node_name,
                  error = %e,
                  "Failed to compress transformer output"
                );
                return Err(crate::execution::ExecutionError::SerializationError {
                  node: node_name,
                  is_deserialization: false,
                  reason: format!("Compression failed: {}", e),
                });
              }
              Err(e) => {
                error!(
                  node = %node_name,
                  error = %e,
                  "Compression task failed"
                );
                return Err(crate::execution::ExecutionError::ExecutionFailed(format!(
                  "Compression task failed: {}",
                  e
                )));
              }
            }
          } else {
            serialized
          };

          if is_fan_out {
            // Fan-out: wrap in Arc for zero-copy sharing of serialized data
            // Use pool if available to reduce allocations
            let shared_bytes: Arc<bytes::Bytes> = if let Some(ref pool) = arc_pool_clone {
              pool.get_or_create(final_bytes)
            } else {
              Arc::new(final_bytes)
            };
            for (port_name, sender) in &output_channels {
              match sender
                .send(ChannelItem::Bytes(shared_bytes.as_ref().clone()))
                .await
              {
                Ok(()) => {
                  send_succeeded += 1;
                }
                Err(_e) => {
                  let paused = *pause_signal.read().await;
                  if paused {
                    return Ok(());
                  }
                  send_failed_count += 1;
                  warn!(
                    node = %node_name,
                    port = %port_name,
                    "Output channel receiver dropped (may be normal in fan-out scenarios)"
                  );
                }
              }
            }
          } else {
            // Single output: send Bytes directly
            let (port_name, sender) = output_channels.iter().next().unwrap();

            // Use batching channel if available, otherwise use regular sender
            let send_result = if let Some(batching_channel) =
              batching_channels.as_ref().and_then(|bc| bc.get(port_name))
            {
              batching_channel.send(ChannelItem::Bytes(final_bytes)).await
            } else {
              sender
                .send(ChannelItem::Bytes(final_bytes))
                .await
                .map_err(|e| crate::execution::ExecutionError::ChannelError {
                  node: node_name.clone(),
                  port: port_name.clone(),
                  is_input: false,
                  reason: format!("Failed to send item: {}", e),
                })
            };

            match send_result {
              Ok(()) => {
                send_succeeded += 1;
              }
              Err(_e) => {
                let paused = *pause_signal.read().await;
                if paused {
                  return Ok(());
                }
                send_failed_count += 1;
                warn!(
                  node = %node_name,
                  port = %port_name,
                  "Output channel receiver dropped"
                );
              }
            }
          }
        }

        // If ALL channels failed (and not shutdown), all downstream nodes have finished
        // Exit gracefully - this is normal behavior when downstream nodes complete
        if send_failed_count > 0 && send_succeeded == 0 {
          // All downstream nodes finished - exit gracefully
          return Ok(());
        }
      }

      Ok(())
    });

    Some(handle)
  }
}

// Implement NodeTrait for ConsumerNode
impl<C, Inputs> NodeTrait for ConsumerNode<C, Inputs>
where
  C: Consumer + Send + Sync + 'static,
  C::Input: std::fmt::Debug + Clone + Send + Sync + DeserializeOwned + 'static,
  Inputs: PortList + Send + Sync + 'static,
  (): ValidateConsumerPorts<C, Inputs>,
{
  fn name(&self) -> &str {
    &self.name
  }

  fn node_kind(&self) -> NodeKind {
    NodeKind::Consumer
  }

  fn input_port_names(&self) -> Vec<String> {
    self.input_port_names.clone()
  }

  fn output_port_names(&self) -> Vec<String> {
    vec![] // Consumers have no output ports
  }

  fn has_input_port(&self, port_name: &str) -> bool {
    self.input_port_names.iter().any(|name| name == port_name)
  }

  fn has_output_port(&self, _port_name: &str) -> bool {
    false // Consumers have no output ports
  }

  fn spawn_execution_task(
    &self,
    input_channels: std::collections::HashMap<String, TypeErasedReceiver>,
    _output_channels: std::collections::HashMap<String, TypeErasedSender>,
    pause_signal: std::sync::Arc<tokio::sync::RwLock<bool>>,
    execution_mode: crate::execution::ExecutionMode,
    _batching_channels: Option<
      std::collections::HashMap<String, std::sync::Arc<crate::batching::BatchingChannel>>,
    >,
    _arc_pool: Option<std::sync::Arc<crate::zero_copy::ArcPool<bytes::Bytes>>>,
  ) -> Option<tokio::task::JoinHandle<Result<(), crate::execution::ExecutionError>>> {
    let consumer = Arc::clone(&self.consumer);

    let handle = tokio::spawn(async move {
      // Create input stream from channels by deserializing
      // Support both single and multiple input channels (fan-in)

      // Extract compression and batching info from execution mode
      let (compression, batching) = match &execution_mode {
        crate::execution::ExecutionMode::Distributed {
          compression: comp,
          batching: batch,
          ..
        } => (*comp, batch.is_some()),
        _ => (None, false),
      };

      // Create StreamWrapper for consumer input (similar to transformer)
      let stream_wrapper: StreamWrapper<C> = if input_channels.is_empty() {
        StreamWrapper::empty()
      } else if input_channels.len() == 1 {
        // Single input: create stream from single receiver
        let (_port_name, receiver) = input_channels.into_iter().next().unwrap();
        StreamWrapper::from_receiver(receiver, compression, batching)
      } else {
        // Multiple inputs: merge streams from all receivers
        StreamWrapper::from_multiple_receivers(input_channels.into_iter(), compression, batching)
      };

      // Convert to C::InputStream using the StreamConverter trait
      let input_stream = stream_wrapper.into_input_stream();

      // Check pause signal before calling consume
      // Use timeout to avoid blocking during shutdown
      let pause_check_result =
        tokio::time::timeout(tokio::time::Duration::from_millis(100), async {
          while *pause_signal.read().await {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
          }
        })
        .await;

      // If timeout occurred and we're still paused, might be shutdown
      // Exit immediately to avoid blocking during shutdown
      if pause_check_result.is_err() && *pause_signal.read().await {
        // Still paused after timeout - likely shutdown, exit gracefully
        return Ok(());
      }

      // Call consumer.consume() with the merged input stream
      // Note: consume() is async and takes &mut self, so we must hold the lock across the await
      {
        let mut consumer_guard = consumer.lock().await;
        consumer_guard.consume(input_stream).await;
      }

      Ok(())
    });

    Some(handle)
  }
}

/// A node that wraps an OutputRouter component.
///
/// OutputRouterNode represents a routing node in the graph that routes a single
/// input stream to multiple output streams based on the router's strategy.
/// It has one input port and multiple output ports specified by the router.
///
/// # Type Parameters
///
/// * `R` - The router type that implements `OutputRouter`
/// * `I` - The input type (items flowing through the router)
/// * `Inputs` - A port tuple representing the input ports (should be `(I,)` for single port)
/// * `Outputs` - A port tuple representing the output ports (e.g., `(I, I)` for two outputs)
///
/// # Example
///
/// ```rust
/// use streamweave::graph::node::OutputRouterNode;
/// use streamweave::graph::control_flow::If;
///
/// let router = If::new(|x: &i32| *x % 2 == 0);
/// let node = OutputRouterNode::from_router(
///     "split".to_string(),
///     router,
/// );
/// ```
pub struct OutputRouterNode<R, I, Inputs, Outputs>
where
  R: crate::router::OutputRouter<I>,
  I: std::fmt::Debug + Clone + Send + Sync + Serialize + 'static,
  Inputs: PortList,
  Outputs: PortList,
{
  name: String,
  router: Arc<Mutex<R>>,
  input_port_names: Vec<String>,
  output_port_names: Vec<String>,
  _phantom: std::marker::PhantomData<(I, Inputs, Outputs)>,
}

impl<R, I, Inputs, Outputs> OutputRouterNode<R, I, Inputs, Outputs>
where
  R: crate::router::OutputRouter<I>,
  I: std::fmt::Debug + Clone + Send + Sync + Serialize + 'static,
  Inputs: PortList,
  Outputs: PortList,
{
  /// Creates a new OutputRouterNode.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of this node
  /// * `router` - The router component to wrap
  /// * `input_port_names` - The names of the input ports (must match Inputs::LEN)
  /// * `output_port_names` - The names of the output ports (must match Outputs::LEN)
  ///
  /// # Returns
  ///
  /// A new `OutputRouterNode` instance.
  ///
  /// # Panics
  ///
  /// Panics if the number of port names doesn't match `Inputs::LEN` or `Outputs::LEN`.
  pub fn new(
    name: String,
    router: R,
    input_port_names: Vec<String>,
    output_port_names: Vec<String>,
  ) -> Self {
    assert_eq!(
      input_port_names.len(),
      Inputs::LEN,
      "Number of input port names ({}) must match Inputs::LEN ({})",
      input_port_names.len(),
      Inputs::LEN
    );
    assert_eq!(
      output_port_names.len(),
      Outputs::LEN,
      "Number of output port names ({}) must match Outputs::LEN ({})",
      output_port_names.len(),
      Outputs::LEN
    );
    Self {
      name,
      router: Arc::new(Mutex::new(router)),
      input_port_names,
      output_port_names,
      _phantom: std::marker::PhantomData,
    }
  }

  /// Creates a new OutputRouterNode from a router.
  ///
  /// This is a convenience method that matches the pattern used by other node types.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of this node
  /// * `router` - The router component to wrap
  /// * `input_port_names` - The names of the input ports (must match Inputs::LEN)
  /// * `output_port_names` - The names of the output ports (must match Outputs::LEN)
  ///
  /// # Returns
  ///
  /// A new `OutputRouterNode` instance.
  pub fn from_router(
    name: String,
    router: R,
    input_port_names: Vec<String>,
    output_port_names: Vec<String>,
  ) -> Self {
    Self::new(name, router, input_port_names, output_port_names)
  }

  /// Returns a reference to the internal router.
  pub fn router(&self) -> Arc<Mutex<R>> {
    Arc::clone(&self.router)
  }
}

// Implement NodeTrait for OutputRouterNode
impl<R, I, Inputs, Outputs> NodeTrait for OutputRouterNode<R, I, Inputs, Outputs>
where
  R: crate::router::OutputRouter<I> + Send + Sync + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + Serialize + DeserializeOwned + 'static,
  Inputs: PortList + Send + Sync + 'static,
  Outputs: PortList + Send + Sync + 'static,
{
  fn name(&self) -> &str {
    &self.name
  }

  fn node_kind(&self) -> NodeKind {
    NodeKind::Transformer // Routers are conceptually transformers
  }

  fn input_port_names(&self) -> Vec<String> {
    self.input_port_names.clone()
  }

  fn output_port_names(&self) -> Vec<String> {
    self.output_port_names.clone()
  }

  fn has_input_port(&self, port_name: &str) -> bool {
    self.input_port_names.iter().any(|name| name == port_name)
  }

  fn has_output_port(&self, port_name: &str) -> bool {
    self.output_port_names.iter().any(|name| name == port_name)
  }

  fn spawn_execution_task(
    &self,
    input_channels: std::collections::HashMap<String, TypeErasedReceiver>,
    output_channels: std::collections::HashMap<String, TypeErasedSender>,
    pause_signal: std::sync::Arc<tokio::sync::RwLock<bool>>,
    execution_mode: crate::execution::ExecutionMode,
    _batching_channels: Option<
      std::collections::HashMap<String, std::sync::Arc<crate::batching::BatchingChannel>>,
    >,
    _arc_pool: Option<std::sync::Arc<crate::zero_copy::ArcPool<bytes::Bytes>>>,
  ) -> Option<tokio::task::JoinHandle<Result<(), crate::execution::ExecutionError>>> {
    let router = Arc::clone(&self.router);
    let node_name = self.name.clone();

    let handle = tokio::spawn(async move {
      // Create input stream from channels
      let (compression, _batching) = match &execution_mode {
        crate::execution::ExecutionMode::Distributed {
          compression: comp,
          batching: batch,
          ..
        } => (*comp, batch.is_some()),
        _ => (None, false),
      };

      // Create input stream from channels by deserializing
      let compression_clone = compression;
      let input_stream: Pin<Box<dyn futures::Stream<Item = I> + Send>> = if input_channels
        .is_empty()
      {
        Box::pin(futures::stream::empty())
      } else if input_channels.len() == 1 {
        let (_port_name, mut receiver) = input_channels.into_iter().next().unwrap();
        Box::pin(stream! {
          while let Some(channel_item) = receiver.recv().await {
            match channel_item {
              crate::channels::ChannelItem::Bytes(bytes) => {
                // Decompress if needed
                let decompressed_bytes = if let Some(algorithm) = compression_clone {
                  let decompressor: Box<dyn crate::compression::Compression> = match algorithm {
                    crate::execution::CompressionAlgorithm::Gzip { level: _ } => {
                      Box::new(crate::compression::GzipCompression::new(1))
                    }
                    crate::execution::CompressionAlgorithm::Zstd { level: _ } => {
                      Box::new(crate::compression::ZstdCompression::new(1))
                    }
                  };
                  match tokio::task::spawn_blocking(move || decompressor.decompress(&bytes)).await {
                    Ok(Ok(decompressed)) => decompressed,
                    Ok(Err(_)) => continue, // Skip corrupted data
                    Err(_) => continue,
                  }
                } else {
                  bytes
                };
                // Deserialize
                match crate::serialization::deserialize::<I>(decompressed_bytes) {
                  Ok(item) => yield item,
                  Err(_) => continue, // Skip deserialization errors
                }
              }
              _ => continue,
            }
          }
        })
      } else {
        // Multiple inputs: merge streams
        let receivers: Vec<_> = input_channels.into_values().collect();
        let compression_clone2 = compression_clone;
        Box::pin(stream! {
          let mut receivers = receivers;
          loop {
            let mut all_done = true;
            for receiver in &mut receivers {
              if let Some(channel_item) = receiver.recv().await {
                all_done = false;
                match channel_item {
                  crate::channels::ChannelItem::Bytes(bytes) => {
                    let decompressed_bytes = if let Some(algorithm) = compression_clone2 {
                      let decompressor: Box<dyn crate::compression::Compression> = match algorithm {
                        crate::execution::CompressionAlgorithm::Gzip { level: _ } => {
                          Box::new(crate::compression::GzipCompression::new(1))
                        }
                        crate::execution::CompressionAlgorithm::Zstd { level: _ } => {
                          Box::new(crate::compression::ZstdCompression::new(1))
                        }
                      };
                      match tokio::task::spawn_blocking(move || decompressor.decompress(&bytes)).await {
                        Ok(Ok(decompressed)) => decompressed,
                        Ok(Err(_)) => continue,
                        Err(_) => continue,
                      }
                    } else {
                      bytes
                    };
                    match crate::serialization::deserialize::<I>(decompressed_bytes) {
                      Ok(item) => yield item,
                      Err(_) => continue,
                    }
                  }
                  _ => continue,
                }
              }
            }
            if all_done {
              break;
            }
          }
        })
      };

      // Route the stream using the router
      let mut router_guard = router.lock().await;
      let output_streams = router_guard.route_stream(input_stream).await;
      drop(router_guard);

      // Send each output stream to the corresponding output channel
      for (port_name, mut output_stream) in output_streams {
        if let Some(sender) = output_channels.get(&port_name) {
          let sender_clone = sender.clone();
          let pause_signal_clone = pause_signal.clone();
          let node_name_clone = node_name.clone();
          let port_name_clone = port_name.clone();
          tokio::spawn(async move {
            while let Some(item) = output_stream.next().await {
              // Check pause signal
              if *pause_signal_clone.read().await {
                return Ok::<(), crate::execution::ExecutionError>(());
              }

              // Serialize and send
              let bytes = serialize(&item).map_err(|e| {
                crate::execution::ExecutionError::SerializationError {
                  node: node_name_clone.clone(),
                  is_deserialization: false,
                  reason: e.to_string(),
                }
              })?;
              sender_clone
                .send(crate::channels::ChannelItem::Bytes(bytes))
                .await
                .map_err(|_| crate::execution::ExecutionError::ChannelError {
                  node: node_name_clone.clone(),
                  port: port_name_clone.clone(),
                  is_input: false,
                  reason: "Channel closed".to_string(),
                })?;
            }
            Ok(())
          });
        }
      }

      Ok(())
    });

    Some(handle)
  }
}

/// A node that wraps an InputRouter component.
///
/// InputRouterNode represents a routing node in the graph that merges multiple
/// input streams into a single output stream based on the router's strategy.
/// It has multiple input ports specified by the router and one output port.
///
/// # Type Parameters
///
/// * `R` - The router type that implements `InputRouter`
/// * `I` - The input type (items flowing through the router)
/// * `Inputs` - A port tuple representing the input ports (e.g., `(I, I)` for two inputs)
/// * `Outputs` - A port tuple representing the output ports (should be `(I,)` for single port)
///
/// # Example
///
/// ```rust
/// use streamweave::graph::node::InputRouterNode;
/// use streamweave::graph::control_flow::Synchronize;
///
/// let router = Synchronize::new(2);
/// let node = InputRouterNode::from_router(
///     "sync".to_string(),
///     router,
/// );
/// ```
pub struct InputRouterNode<R, I, Inputs, Outputs>
where
  R: crate::router::InputRouter<I>,
  I: std::fmt::Debug + Clone + Send + Sync + Serialize + 'static,
  Inputs: PortList,
  Outputs: PortList,
{
  name: String,
  router: Arc<Mutex<R>>,
  input_port_names: Vec<String>,
  output_port_names: Vec<String>,
  _phantom: std::marker::PhantomData<(I, Inputs, Outputs)>,
}

impl<R, I, Inputs, Outputs> InputRouterNode<R, I, Inputs, Outputs>
where
  R: crate::router::InputRouter<I>,
  I: std::fmt::Debug + Clone + Send + Sync + Serialize + 'static,
  Inputs: PortList,
  Outputs: PortList,
{
  /// Creates a new InputRouterNode.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of this node
  /// * `router` - The router component to wrap
  /// * `input_port_names` - The names of the input ports (must match Inputs::LEN)
  /// * `output_port_names` - The names of the output ports (must match Outputs::LEN)
  ///
  /// # Returns
  ///
  /// A new `InputRouterNode` instance.
  ///
  /// # Panics
  ///
  /// Panics if the number of port names doesn't match `Inputs::LEN` or `Outputs::LEN`.
  pub fn new(
    name: String,
    router: R,
    input_port_names: Vec<String>,
    output_port_names: Vec<String>,
  ) -> Self {
    assert_eq!(
      input_port_names.len(),
      Inputs::LEN,
      "Number of input port names ({}) must match Inputs::LEN ({})",
      input_port_names.len(),
      Inputs::LEN
    );
    assert_eq!(
      output_port_names.len(),
      Outputs::LEN,
      "Number of output port names ({}) must match Outputs::LEN ({})",
      output_port_names.len(),
      Outputs::LEN
    );
    Self {
      name,
      router: Arc::new(Mutex::new(router)),
      input_port_names,
      output_port_names,
      _phantom: std::marker::PhantomData,
    }
  }

  /// Creates a new InputRouterNode from a router.
  ///
  /// This is a convenience method that matches the pattern used by other node types.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of this node
  /// * `router` - The router component to wrap
  /// * `input_port_names` - The names of the input ports (must match Inputs::LEN)
  /// * `output_port_names` - The names of the output ports (must match Outputs::LEN)
  ///
  /// # Returns
  ///
  /// A new `InputRouterNode` instance.
  pub fn from_router(
    name: String,
    router: R,
    input_port_names: Vec<String>,
    output_port_names: Vec<String>,
  ) -> Self {
    Self::new(name, router, input_port_names, output_port_names)
  }

  /// Returns a reference to the internal router.
  pub fn router(&self) -> Arc<Mutex<R>> {
    Arc::clone(&self.router)
  }
}

// Implement NodeTrait for InputRouterNode
impl<R, I, Inputs, Outputs> NodeTrait for InputRouterNode<R, I, Inputs, Outputs>
where
  R: crate::router::InputRouter<I> + Send + Sync + 'static,
  I: std::fmt::Debug + Clone + Send + Sync + Serialize + DeserializeOwned + 'static,
  Inputs: PortList + Send + Sync + 'static,
  Outputs: PortList + Send + Sync + 'static,
{
  fn name(&self) -> &str {
    &self.name
  }

  fn node_kind(&self) -> NodeKind {
    NodeKind::Transformer // Routers are conceptually transformers
  }

  fn input_port_names(&self) -> Vec<String> {
    self.input_port_names.clone()
  }

  fn output_port_names(&self) -> Vec<String> {
    self.output_port_names.clone()
  }

  fn has_input_port(&self, port_name: &str) -> bool {
    self.input_port_names.iter().any(|name| name == port_name)
  }

  fn has_output_port(&self, port_name: &str) -> bool {
    self.output_port_names.iter().any(|name| name == port_name)
  }

  fn spawn_execution_task(
    &self,
    input_channels: std::collections::HashMap<String, TypeErasedReceiver>,
    output_channels: std::collections::HashMap<String, TypeErasedSender>,
    pause_signal: std::sync::Arc<tokio::sync::RwLock<bool>>,
    execution_mode: crate::execution::ExecutionMode,
    _batching_channels: Option<
      std::collections::HashMap<String, std::sync::Arc<crate::batching::BatchingChannel>>,
    >,
    _arc_pool: Option<std::sync::Arc<crate::zero_copy::ArcPool<bytes::Bytes>>>,
  ) -> Option<tokio::task::JoinHandle<Result<(), crate::execution::ExecutionError>>> {
    let router = Arc::clone(&self.router);
    let node_name = self.name.clone();

    let handle = tokio::spawn(async move {
      // Create input streams from channels
      let (compression, _batching) = match &execution_mode {
        crate::execution::ExecutionMode::Distributed {
          compression: comp,
          batching: batch,
          ..
        } => (*comp, batch.is_some()),
        _ => (None, false),
      };

      // Create input streams from channels by deserializing
      let mut input_streams = Vec::new();
      for (port_name, mut receiver) in input_channels {
        let compression_clone = compression;
        let stream: Pin<Box<dyn futures::Stream<Item = I> + Send>> = Box::pin(stream! {
          while let Some(channel_item) = receiver.recv().await {
            match channel_item {
              crate::channels::ChannelItem::Bytes(bytes) => {
                // Decompress if needed
                let decompressed_bytes = if let Some(algorithm) = compression_clone {
                  let decompressor: Box<dyn crate::compression::Compression> = match algorithm {
                    crate::execution::CompressionAlgorithm::Gzip { level: _ } => {
                      Box::new(crate::compression::GzipCompression::new(1))
                    }
                    crate::execution::CompressionAlgorithm::Zstd { level: _ } => {
                      Box::new(crate::compression::ZstdCompression::new(1))
                    }
                  };
                  match tokio::task::spawn_blocking(move || decompressor.decompress(&bytes)).await {
                    Ok(Ok(decompressed)) => decompressed,
                    Ok(Err(_)) => continue,
                    Err(_) => continue,
                  }
                } else {
                  bytes
                };
                // Deserialize
                match crate::serialization::deserialize::<I>(decompressed_bytes) {
                  Ok(item) => yield item,
                  Err(_) => continue,
                }
              }
              _ => continue,
            }
          }
        });
        input_streams.push((port_name, stream));
      }

      // Route the streams using the router
      let mut router_guard = router.lock().await;
      let output_stream = router_guard.route_streams(input_streams).await;
      drop(router_guard);

      // Send the output stream to the output channel
      // Get the first (and typically only) output port name
      if let Some(output_port_name) = output_channels.keys().next()
        && let Some(sender) = output_channels.get(output_port_name)
      {
        let mut output_stream = output_stream;
        while let Some(item) = output_stream.next().await {
          // Check pause signal
          if *pause_signal.read().await {
            return Ok(());
          }

          // Serialize and send
          let bytes =
            serialize(&item).map_err(|e| crate::execution::ExecutionError::SerializationError {
              node: node_name.clone(),
              is_deserialization: false,
              reason: e.to_string(),
            })?;
          sender
            .send(crate::channels::ChannelItem::Bytes(bytes))
            .await
            .map_err(|_| crate::execution::ExecutionError::ChannelError {
              node: node_name.clone(),
              port: output_port_name.clone(),
              is_input: false,
              reason: "Channel closed".to_string(),
            })?;
        }
      }

      Ok(())
    });

    Some(handle)
  }
}
