//! # Transformer Trait
//!
//! This module defines the `Transformer` trait for components that transform data streams
//! in the StreamWeave framework. Transformers process items as they flow through pipelines,
//! performing operations like filtering, mapping, aggregation, or any other transformation.
//!
//! ## Overview
//!
//! The Transformer trait provides:
//!
//! - **Stream Transformation**: Async transformation of input streams into output streams
//! - **Error Handling**: Configurable error strategies per transformer
//! - **Component Information**: Name and type information for debugging
//! - **Configuration**: TransformerConfig for error strategy and naming
//! - **Port System**: Type-safe input and output port definitions
//!
//! ## Universal Message Model
//!
//! **All transformers work with `Message<T>` where `T` is the payload type.**
//! Both `Transformer::Input` and `Transformer::Output` are `Message<T>` types.
//! This enables:
//!
//! - Message ID preservation through transformations
//! - Metadata preservation and modification
//! - End-to-end message tracking
//!
//! ## Example
//!
//! ```rust
//! use crate::transformer::Transformer;
//! use crate::transformers::MapTransformer;
//! use crate::message::Message;
//! use futures::stream;
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut transformer = MapTransformer::<_, i32, i32>::new(|x| x * 2);
//! let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));
//!
//! let mut output_stream = transformer.transform(input_stream).await;
//! while let Some(msg) = output_stream.next().await {
//!     // msg is Message<i32> with doubled value
//!     println!("Transformed: {:?}", msg.payload());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Key Concepts
//!
//! - **Transformer**: A component that transforms streams of `Message<T>`
//! - **TransformerConfig**: Configuration including error strategy and component name
//! - **Error Strategy**: How to handle errors during transformation (Stop, Skip, Retry, Custom)
//! - **InputPorts/OutputPorts**: Type-level port definitions (default to single port each)
//!
//! ## Usage
//!
//! Transformers are the core processing components in StreamWeave. They can perform
//! any transformation on message streams while preserving message IDs and allowing
//! metadata modification. Common transformers include map, filter, aggregate, and
//! custom business logic transformations.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::port::PortList;
use crate::{input::Input, output::Output};
use async_trait::async_trait;

/// Helper trait for providing default port types for Transformers.
///
/// This trait provides default implementations of the `InputPorts` and `OutputPorts` associated types.
/// All Transformers automatically get `InputPorts = (Self::Input,)` and `OutputPorts = (Self::Output,)`
/// unless they explicitly override them.
pub trait TransformerPorts: Transformer
where
  Self::Input: std::fmt::Debug + Clone + Send + Sync,
{
  /// The default input port tuple type (single port with the transformer's input type).
  type DefaultInputPorts: PortList;
  /// The default output port tuple type (single port with the transformer's output type).
  type DefaultOutputPorts: PortList;
}

/// Blanket implementation: all Transformers get default single-port inputs and outputs.
impl<T> TransformerPorts for T
where
  T: Transformer,
  T::Input: std::fmt::Debug + Clone + Send + Sync,
{
  type DefaultInputPorts = (T::Input,);
  type DefaultOutputPorts = (T::Output,);
}

/// Configuration for transformers, including error handling strategy and naming.
///
/// This struct holds configuration options that can be applied to any transformer,
/// allowing customization of error handling behavior and component identification.
///
/// The configuration works with `Message<T>` where `M` is the message type.
#[derive(Debug, Clone, PartialEq)]
pub struct TransformerConfig<M: std::fmt::Debug + Clone + Send + Sync> {
  /// The error handling strategy to use when errors occur.
  /// This works with `Message<T>` types.
  pub error_strategy: ErrorStrategy<M>,
  /// Optional name for identifying this transformer in logs and metrics.
  pub name: Option<String>,
}

impl<M: std::fmt::Debug + Clone + Send + Sync> Default for TransformerConfig<M> {
  fn default() -> Self {
    Self {
      error_strategy: ErrorStrategy::Stop,
      name: None,
    }
  }
}

impl<M: std::fmt::Debug + Clone + Send + Sync> TransformerConfig<M> {
  /// Sets the error handling strategy for this transformer configuration.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use (works with `Message<T>`).
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<M>) -> Self {
    self.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer configuration.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.name = Some(name);
    self
  }

  /// Returns the current error handling strategy.
  pub fn error_strategy(&self) -> ErrorStrategy<M> {
    self.error_strategy.clone()
  }

  /// Returns the current name, if set.
  pub fn name(&self) -> Option<String> {
    self.name.clone()
  }
}

/// Trait for components that transform data streams.
///
/// Transformers process items as they flow through the pipeline. They can
/// filter, map, aggregate, or perform any other transformation on stream items.
///
/// ## Universal Message Model
///
/// **All transformers work with `Message<T>` where `T` is the payload type.**
/// Both `Transformer::Input` and `Transformer::Output` are `Message<T>` types.
/// This enables:
/// - Message ID preservation through transformations
/// - Metadata preservation and modification
/// - End-to-end message tracking
///
/// ## Working with Messages
///
/// When implementing a transformer, you receive and produce `Message<T>`:
///
/// ```rust,ignore
/// use crate::{Transformer, Input, Output, TransformerConfig};
/// use crate::message::Message;
/// use futures::StreamExt;
/// use std::pin::Pin;
/// use tokio_stream::Stream;
///
/// struct DoubleTransformer {
///     config: TransformerConfig<Message<i32>>,
/// }
///
/// impl Input for DoubleTransformer {
///     type Input = Message<i32>;
///     type InputStream = Pin<Box<dyn Stream<Item = Message<i32>> + Send>>;
/// }
///
/// impl Output for DoubleTransformer {
///     type Output = Message<i32>;
///     type OutputStream = Pin<Box<dyn Stream<Item = Message<i32>> + Send>>;
/// }
///
/// #[async_trait::async_trait]
/// impl Transformer for DoubleTransformer {
///     type InputPorts = (Message<i32>,);
///     type OutputPorts = (Message<i32>,);
///
///     async fn transform(&mut self, stream: Self::InputStream) -> Self::OutputStream {
///         Box::pin(stream.map(|msg| {
///             // Access payload
///             let payload = msg.payload().clone();
///             // Preserve ID and metadata
///             let id = msg.id().clone();
///             let metadata = msg.metadata().clone();
///             // Create new message with transformed payload
///             Message::with_metadata(payload * 2, id, metadata)
///         }))
///     }
///
///     fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>) {
///         self.config = config;
///     }
///
///     fn get_config_impl(&self) -> &TransformerConfig<Self::Input> {
///         &self.config
///     }
///
///     fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input> {
///         &mut self.config
///     }
/// }
/// ```
///
/// ## Message Operations
///
/// Common operations when transforming messages:
/// - `message.payload()` - Access the payload data
/// - `message.id()` - Access the message ID (preserve through transformations)
/// - `message.metadata()` - Access or modify metadata
/// - `message.map(f)` - Transform payload while preserving ID and metadata
///
/// ## Message Wrapping
///
/// Transformers work with raw types (`T::Input` -> `T::Output`). When using `TransformerNode`
/// in graphs, messages are automatically unwrapped before transformation, and the output is
/// wrapped back into `Message<T::Output>` with preserved IDs and metadata.
///
/// # Implementations
///
/// Common transformer implementations include:
/// - `MapTransformer` - Transforms each item
/// - `FilterTransformer` - Filters items based on a predicate
/// - `BatchTransformer` - Groups items into batches
/// - `RateLimitTransformer` - Controls throughput
/// - `RetryTransformer` - Retries failed items
#[async_trait]
pub trait Transformer: Input + Output
where
  Self::Input: std::fmt::Debug + Clone + Send + Sync,
{
  /// The input port tuple type for this transformer.
  ///
  /// This associated type specifies the port tuple that represents this transformer's
  /// inputs in the graph API. By default, transformers have a single input port
  /// containing their input type: `(Self::Input,)`.
  ///
  /// For multi-port transformers, override this type to specify a tuple with multiple
  /// input types, e.g., `(i32, String)` for two inputs.
  type InputPorts: PortList;

  /// The output port tuple type for this transformer.
  ///
  /// This associated type specifies the port tuple that represents this transformer's
  /// outputs in the graph API. By default, transformers have a single output port
  /// containing their output type: `(Self::Output,)`.
  ///
  /// For multi-port transformers, override this type to specify a tuple with multiple
  /// output types, e.g., `(i32, String)` for two outputs.
  type OutputPorts: PortList;

  /// Transforms a stream of input items into a stream of output items.
  ///
  /// This method is called by the pipeline to process items. The transformer
  /// receives items from the previous component and produces transformed items
  /// for the next component.
  ///
  /// # Arguments
  ///
  /// * `input` - The input stream to transform (yields `Message<T>`)
  ///
  /// # Returns
  ///
  /// A stream that yields transformed items of type `Self::Output`, which is `Message<U>`
  /// where `U` is the output payload type.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use crate::{Transformer, Input, Output};
  /// use crate::message::Message;
  /// use futures::StreamExt;
  ///
  /// // In your Transformer implementation:
  /// // async fn transform(&mut self, stream: Self::InputStream) -> Self::OutputStream {
  /// //     Box::pin(stream.map(|msg| {
  /// //         let payload = msg.payload().clone();
  /// //         let id = msg.id().clone();
  /// //         let metadata = msg.metadata().clone();
  /// //         // Transform payload while preserving ID and metadata
  /// //         Message::with_metadata(payload * 2, id, metadata)
  /// //     }))
  /// // }
  /// ```
  ///
  /// # Message Preservation
  ///
  /// It's important to preserve message IDs and metadata through transformations
  /// to maintain end-to-end traceability. Always clone the ID and metadata from
  /// the input message when creating the output message.
  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream;

  /// Creates a new transformer instance with the given configuration.
  ///
  /// This method clones the transformer and applies the provided configuration.
  ///
  /// # Arguments
  ///
  /// * `config` - The `TransformerConfig` to apply.
  ///
  /// # Returns
  ///
  /// A new transformer instance with the specified configuration.
  #[must_use]
  fn with_config(&self, config: TransformerConfig<Self::Input>) -> Self
  where
    Self: Sized + Clone,
  {
    let mut this = self.clone();
    this.set_config(config);
    this
  }

  /// Sets the configuration for this transformer.
  ///
  /// # Arguments
  ///
  /// * `config` - The new `TransformerConfig` to apply.
  fn set_config(&mut self, config: TransformerConfig<Self::Input>) {
    self.set_config_impl(config);
  }

  /// Returns a reference to the transformer's configuration.
  ///
  /// # Returns
  ///
  /// A reference to the `TransformerConfig` for this transformer.
  fn config(&self) -> &TransformerConfig<Self::Input> {
    self.get_config_impl()
  }

  /// Returns a mutable reference to the transformer's configuration.
  ///
  /// # Returns
  ///
  /// A mutable reference to the `TransformerConfig` for this transformer.
  fn config_mut(&mut self) -> &mut TransformerConfig<Self::Input> {
    self.get_config_mut_impl()
  }

  /// Sets the name for this transformer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this transformer.
  ///
  /// # Returns
  ///
  /// The transformer instance with the updated name.
  #[must_use]
  fn with_name(mut self, name: String) -> Self
  where
    Self: Sized,
  {
    let config = self.get_config_impl().clone();
    self.set_config(TransformerConfig {
      error_strategy: config.error_strategy,
      name: Some(name),
    });
    self
  }

  /// Handles an error that occurred during stream processing.
  ///
  /// This method determines the appropriate `ErrorAction` based on the
  /// transformer's configured `ErrorStrategy`.
  ///
  /// # Arguments
  ///
  /// * `error` - The `StreamError` that occurred.
  ///
  /// # Returns
  ///
  /// The `ErrorAction` to take in response to the error.
  fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction {
    match self.config().error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  /// Creates an error context for error reporting.
  ///
  /// This method constructs an `ErrorContext` with the current timestamp,
  /// the item that caused the error (if any), and component information.
  ///
  /// **Note**: The item is `Message<T>`, so message IDs and metadata are available
  /// through the item. For example: `context.item.as_ref().map(|msg| msg.id())`.
  ///
  /// # Arguments
  ///
  /// * `item` - The message that caused the error, if available.
  ///
  /// # Returns
  ///
  /// An `ErrorContext` containing error details, including the full `Message<T>`
  /// which provides access to message ID and metadata.
  fn create_error_context(&self, item: Option<Self::Input>) -> ErrorContext<Self::Input> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: self.component_info().type_name,
    }
  }

  /// Returns information about the component for error reporting.
  ///
  /// This includes the component's name and type. For message-level information
  /// (message IDs, metadata), use `create_error_context()` which includes the
  /// full `Message<T>` in the error context.
  ///
  /// # Returns
  ///
  /// A `ComponentInfo` struct containing details about the transformer.
  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config()
        .name()
        .unwrap_or_else(|| "transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }

  /// Sets the configuration implementation.
  ///
  /// This method must be implemented by each transformer to store the configuration.
  ///
  /// # Arguments
  ///
  /// * `config` - The `TransformerConfig` to store.
  fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>);

  /// Returns a reference to the configuration implementation.
  ///
  /// This method must be implemented by each transformer to return its stored configuration.
  ///
  /// # Returns
  ///
  /// A reference to the transformer's `TransformerConfig`.
  fn get_config_impl(&self) -> &TransformerConfig<Self::Input>;

  /// Returns a mutable reference to the configuration implementation.
  ///
  /// This method must be implemented by each transformer to return a mutable reference
  /// to its stored configuration.
  ///
  /// # Returns
  ///
  /// A mutable reference to the transformer's `TransformerConfig`.
  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input>;
}
