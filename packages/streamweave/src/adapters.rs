//! Adapter patterns for ergonomic usage with raw types.
//!
//! This module provides adapters that allow users to work with raw types (T) while
//! internally the system uses `Message<T>`. This provides backward compatibility
//! and ergonomic APIs for simple use cases.
//!
//! # Overview
//!
//! The adapters in this module automatically wrap raw types into `Message<T>` and
//! unwrap `Message<T>` back to raw types, allowing users to work with simple types
//! while the system uses messages internally.
//!
//! # Example
//!
//! ```rust,no_run
//! use streamweave::adapters::MessageWrapper;
//! use streamweave::producer::Producer;
//! use streamweave::message::{Message, MessageId};
//!
//! // Wrap a producer that produces raw types
//! // let raw_producer = /* some producer that produces i32 */;
//! // let wrapped = MessageWrapper::new(raw_producer);
//! // wrapped now implements Producer with Output = Message<i32>
//! ```

use crate::consumer::{Consumer, ConsumerConfig};
use crate::input::Input;
use crate::message::{IdGenerator, Message, MessageId, UuidGenerator};
use crate::output::Output;
use crate::producer::{Producer, ProducerConfig};
use crate::transformer::{Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use std::pin::Pin;
use std::sync::Arc;

/// Trait for components that produce raw payload types (before message wrapping).
///
/// This trait is used by adapters to wrap raw producers into message-producing producers.
/// Implementations of this trait produce raw types `T` which are then automatically
/// wrapped into `Message<T>` by the `MessageWrapper` adapter.
pub trait RawProducer {
  /// The raw payload type produced by this producer.
  type Payload: std::fmt::Debug + Clone + Send + Sync + 'static;

  /// Produces a stream of raw payload items.
  ///
  /// # Returns
  ///
  /// A stream that yields items of type `Self::Payload`.
  fn produce_raw(&mut self) -> Pin<Box<dyn Stream<Item = Self::Payload> + Send>>;
}

/// Adapter that wraps a raw producer to produce `Message<T>`.
///
/// This adapter takes a producer that produces raw types `T` and automatically
/// wraps each item into `Message<T>` with generated IDs and metadata.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::adapters::{MessageWrapper, RawProducer};
/// use streamweave::producer::Producer;
/// use streamweave::message::Message;
/// use std::pin::Pin;
/// use tokio_stream::Stream;
///
/// struct MyRawProducer {
///     items: Vec<i32>,
/// }
///
/// impl RawProducer for MyRawProducer {
///     type Payload = i32;
///     fn produce_raw(&mut self) -> Pin<Box<dyn Stream<Item = i32> + Send>> {
///         Box::pin(futures::stream::iter(self.items.clone()))
///     }
/// }
///
/// let raw = MyRawProducer { items: vec![1, 2, 3] };
/// let wrapped = MessageWrapper::new(raw);
/// // wrapped now implements Producer with Output = Message<i32>
/// ```
pub struct MessageWrapper<P: RawProducer> {
  inner: P,
  id_generator: Arc<dyn IdGenerator>,
  config: ProducerConfig<Message<P::Payload>>,
}

impl<P: RawProducer> MessageWrapper<P> {
  /// Create a new `MessageWrapper` that wraps a raw producer.
  ///
  /// # Arguments
  ///
  /// * `producer` - The raw producer to wrap
  ///
  /// # Returns
  ///
  /// A new `MessageWrapper` that produces `Message<P::Payload>`.
  #[must_use]
  pub fn new(producer: P) -> Self {
    Self {
      inner: producer,
      id_generator: Arc::new(UuidGenerator::new()),
      config: ProducerConfig::default(),
    }
  }

  /// Create a new `MessageWrapper` with a custom ID generator.
  ///
  /// # Arguments
  ///
  /// * `producer` - The raw producer to wrap
  /// * `id_generator` - The ID generator to use for creating message IDs
  ///
  /// # Returns
  ///
  /// A new `MessageWrapper` with the specified ID generator.
  #[must_use]
  pub fn with_id_generator(producer: P, id_generator: Arc<dyn IdGenerator>) -> Self {
    Self {
      inner: producer,
      id_generator,
      config: ProducerConfig::default(),
    }
  }

  /// Get a reference to the inner raw producer.
  ///
  /// # Returns
  ///
  /// A reference to the wrapped producer.
  pub fn inner(&self) -> &P {
    &self.inner
  }

  /// Get a mutable reference to the inner raw producer.
  ///
  /// # Returns
  ///
  /// A mutable reference to the wrapped producer.
  pub fn inner_mut(&mut self) -> &mut P {
    &mut self.inner
  }

  /// Consume the wrapper and return the inner producer.
  ///
  /// # Returns
  ///
  /// The wrapped producer.
  pub fn into_inner(self) -> P {
    self.inner
  }
}

impl<P: RawProducer> Producer for MessageWrapper<P> {
  type OutputPorts = (Message<P::Payload>,);

  fn produce(&mut self) -> Self::OutputStream {
    let id_gen = Arc::clone(&self.id_generator);
    let raw_stream = self.inner.produce_raw();

    // Wrap each raw item into a Message
    Box::pin(raw_stream.map(move |payload| Message::new(payload, id_gen.next_id())))
  }

  fn set_config_impl(&mut self, config: ProducerConfig<Message<P::Payload>>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<Message<P::Payload>> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<Message<P::Payload>> {
    &mut self.config
  }
}

impl<P: RawProducer> Output for MessageWrapper<P> {
  type Output = Message<P::Payload>;
  type OutputStream = Pin<Box<dyn Stream<Item = Message<P::Payload>> + Send>>;
}

/// Type alias for the complex return type of `RawTransformer::transform_raw`.
type RawTransformerFuture<T> =
  Pin<Box<dyn std::future::Future<Output = Pin<Box<dyn Stream<Item = T> + Send>>> + Send>>;

/// Trait for transformers that work with raw payload types (before message wrapping).
///
/// This trait is used by adapters to wrap raw transformers into message-transforming transformers.
/// Implementations of this trait transform raw types `T` to `U`, which are then automatically
/// wrapped/unwrapped from `Message<T>` and `Message<U>` by the `PayloadExtractor` adapter.
pub trait RawTransformer: Send {
  /// The raw input payload type.
  type InputPayload: std::fmt::Debug + Clone + Send + Sync + 'static;

  /// The raw output payload type.
  type OutputPayload: std::fmt::Debug + Clone + Send + Sync + 'static;

  /// Transforms a stream of raw input payloads into a stream of raw output payloads.
  ///
  /// # Arguments
  ///
  /// * `input` - The input stream of raw payloads
  ///
  /// # Returns
  ///
  /// A stream that yields transformed payloads of type `Self::OutputPayload`.
  ///
  /// The returned future must be `Send` to allow cross-thread execution.
  fn transform_raw(
    &mut self,
    input: Pin<Box<dyn Stream<Item = Self::InputPayload> + Send>>,
  ) -> RawTransformerFuture<Self::OutputPayload>;
}

/// Adapter that extracts payloads from `Message<T>`, passes them to a raw transformer,
/// and wraps the output back into `Message<U>` while preserving message metadata.
///
/// This adapter allows transformers to work with raw payload types while the system
/// uses `Message<T>` internally. The adapter preserves message IDs and metadata from
/// the input messages.
///
/// # Example
///
/// ```rust,ignore
/// use streamweave::adapters::{PayloadExtractor, RawTransformer};
/// use streamweave::message::Message;
/// use std::pin::Pin;
/// use tokio_stream::Stream;
/// use futures::StreamExt;
///
/// struct MyRawTransformer;
///
/// impl RawTransformer for MyRawTransformer {
///     type InputPayload = i32;
///     type OutputPayload = i64;
///     fn transform_raw(
///         &mut self,
///         input: Pin<Box<dyn Stream<Item = i32> + Send>>,
///     ) -> Pin<Box<dyn std::future::Future<Output = Pin<Box<dyn Stream<Item = i64> + Send>>> + Send>> {
///         Box::pin(async move {
///             let mapped: Pin<Box<dyn Stream<Item = i64> + Send>> = Box::pin(input.map(|x| x as i64));
///             mapped
///         })
///     }
/// }
///
/// let raw = MyRawTransformer;
/// let wrapped = PayloadExtractor::new(raw);
/// // wrapped now implements Transformer with Input = Message<i32>, Output = Message<i64>
/// ```
pub struct PayloadExtractor<T: RawTransformer> {
  inner: T,
  config: TransformerConfig<Message<T::InputPayload>>,
}

impl<T: RawTransformer> PayloadExtractor<T> {
  /// Create a new `PayloadExtractor` that wraps a raw transformer.
  ///
  /// # Arguments
  ///
  /// * `transformer` - The raw transformer to wrap
  ///
  /// # Returns
  ///
  /// A new `PayloadExtractor` that works with `Message<T>` and `Message<U>`.
  #[must_use]
  pub fn new(transformer: T) -> Self {
    Self {
      inner: transformer,
      config: TransformerConfig::default(),
    }
  }

  /// Get a reference to the inner raw transformer.
  ///
  /// # Returns
  ///
  /// A reference to the wrapped transformer.
  pub fn inner(&self) -> &T {
    &self.inner
  }

  /// Get a mutable reference to the inner raw transformer.
  ///
  /// # Returns
  ///
  /// A mutable reference to the wrapped transformer.
  pub fn inner_mut(&mut self) -> &mut T {
    &mut self.inner
  }

  /// Consume the wrapper and return the inner transformer.
  ///
  /// # Returns
  ///
  /// The wrapped transformer.
  pub fn into_inner(self) -> T {
    self.inner
  }
}

#[async_trait]
impl<T: RawTransformer> Transformer for PayloadExtractor<T> {
  type InputPorts = (Message<T::InputPayload>,);
  type OutputPorts = (Message<T::OutputPayload>,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    // Extract payloads from messages, preserving message metadata
    // We collect messages first to preserve metadata, then transform payloads
    // This is a simple approach - in production, you might want a more sophisticated
    // approach that handles 1-to-many or many-to-1 transformations
    use futures::StreamExt;

    // Collect all messages to preserve metadata
    let messages: Vec<Message<T::InputPayload>> = input.collect().await;

    // Extract payloads and metadata separately
    let mut ids = Vec::new();
    let mut payloads = Vec::new();
    let mut metadatas = Vec::new();

    for msg in messages {
      let (id, payload, metadata) = msg.into_parts();
      ids.push(id);
      payloads.push(payload);
      metadatas.push(metadata);
    }

    // Create stream of payloads
    let payload_stream = Box::pin(futures::stream::iter(payloads));

    // Transform the payloads using the raw transformer
    // Move the inner transformer to avoid borrowing issues
    let transformed_payloads = {
      let inner = &mut self.inner;
      inner.transform_raw(payload_stream).await
    };

    // Wrap transformed payloads back into messages, preserving original IDs and metadata
    // For 1-to-1 transformations, we pair each output with the corresponding input metadata
    let ids = ids;
    let metadatas = metadatas;
    let mut id_iter = ids.into_iter();
    let mut metadata_iter = metadatas.into_iter();

    Box::pin(transformed_payloads.map(move |payload| {
      // Use the next message's ID and metadata, or create new ones if exhausted
      if let (Some(id), Some(metadata)) = (id_iter.next(), metadata_iter.next()) {
        Message::with_metadata(payload, id, metadata)
      } else {
        // If we have more outputs than inputs, create new messages
        Message::new(payload, MessageId::new_uuid())
      }
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Message<T::InputPayload>>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Message<T::InputPayload>> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Message<T::InputPayload>> {
    &mut self.config
  }
}

impl<T: RawTransformer> Input for PayloadExtractor<T> {
  type Input = Message<T::InputPayload>;
  type InputStream = Pin<Box<dyn Stream<Item = Message<T::InputPayload>> + Send>>;
}

impl<T: RawTransformer> Output for PayloadExtractor<T> {
  type Output = Message<T::OutputPayload>;
  type OutputStream = Pin<Box<dyn Stream<Item = Message<T::OutputPayload>> + Send>>;
}

/// Trait for consumers that work with raw payload types (before message wrapping).
///
/// This trait is used by adapters to wrap raw consumers into message-consuming consumers.
/// Implementations of this trait consume raw types `T`, which are automatically extracted
/// from `Message<T>` by the `PayloadExtractor` adapter.
pub trait RawConsumer: Send {
  /// The raw input payload type.
  type Payload: std::fmt::Debug + Clone + Send + Sync + 'static;

  /// Consumes a stream of raw input payloads.
  ///
  /// # Arguments
  ///
  /// * `stream` - The input stream of raw payloads
  ///
  /// The returned future must be `Send` to allow cross-thread execution.
  fn consume_raw(
    &mut self,
    stream: Pin<Box<dyn Stream<Item = Self::Payload> + Send>>,
  ) -> Pin<Box<dyn std::future::Future<Output = ()> + Send>>;
}

/// Adapter that extracts payloads from `Message<T>` before passing them to a raw consumer.
///
/// This adapter allows consumers to work with raw payload types while the system
/// uses `Message<T>` internally. The adapter automatically extracts payloads from
/// messages before passing them to the raw consumer.
///
/// # Example
///
/// ```rust,ignore
/// use streamweave::adapters::{PayloadExtractorConsumer, RawConsumer};
/// use streamweave::message::Message;
/// use std::pin::Pin;
/// use tokio_stream::Stream;
///
/// struct MyRawConsumer {
///     items: Vec<i32>,
/// }
///
/// impl RawConsumer for MyRawConsumer {
///     type Payload = i32;
///     fn consume_raw(
///         &mut self,
///         stream: Pin<Box<dyn Stream<Item = i32> + Send>>,
///     ) -> Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
///         let items = &mut self.items;
///         Box::pin(async move {
///             use futures::StreamExt;
///             stream.for_each(|item| async move {
///                 items.push(item);
///             }).await;
///         })
///     }
/// }
///
/// let raw = MyRawConsumer { items: vec![] };
/// let wrapped = PayloadExtractorConsumer::new(raw);
/// // wrapped now implements Consumer with Input = Message<i32>
/// ```
pub struct PayloadExtractorConsumer<C: RawConsumer> {
  inner: C,
  config: ConsumerConfig<Message<C::Payload>>,
}

impl<C: RawConsumer> PayloadExtractorConsumer<C> {
  /// Create a new `PayloadExtractorConsumer` that wraps a raw consumer.
  ///
  /// # Arguments
  ///
  /// * `consumer` - The raw consumer to wrap
  ///
  /// # Returns
  ///
  /// A new `PayloadExtractorConsumer` that works with `Message<T>`.
  #[must_use]
  pub fn new(consumer: C) -> Self {
    Self {
      inner: consumer,
      config: ConsumerConfig::default(),
    }
  }

  /// Get a reference to the inner raw consumer.
  ///
  /// # Returns
  ///
  /// A reference to the wrapped consumer.
  pub fn inner(&self) -> &C {
    &self.inner
  }

  /// Get a mutable reference to the inner raw consumer.
  ///
  /// # Returns
  ///
  /// A mutable reference to the wrapped consumer.
  pub fn inner_mut(&mut self) -> &mut C {
    &mut self.inner
  }

  /// Consume the wrapper and return the inner consumer.
  ///
  /// # Returns
  ///
  /// The wrapped consumer.
  pub fn into_inner(self) -> C {
    self.inner
  }
}

#[async_trait]
impl<C: RawConsumer> Consumer for PayloadExtractorConsumer<C> {
  type InputPorts = (Message<C::Payload>,);

  async fn consume(&mut self, stream: Self::InputStream) {
    // Extract payloads from messages
    use futures::StreamExt;

    let payload_stream = Box::pin(stream.map(|msg| {
      // Extract just the payload, discarding message ID and metadata
      // (consumers typically only need the payload)
      msg.into_payload()
    }));

    // Consume the payloads using the raw consumer
    // Move the inner consumer to avoid borrowing issues
    {
      let inner = &mut self.inner;
      inner.consume_raw(payload_stream).await;
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<Message<C::Payload>>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<Message<C::Payload>> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<Message<C::Payload>> {
    &mut self.config
  }
}

impl<C: RawConsumer> Input for PayloadExtractorConsumer<C> {
  type Input = Message<C::Payload>;
  type InputStream = Pin<Box<dyn Stream<Item = Message<C::Payload>> + Send>>;
}
