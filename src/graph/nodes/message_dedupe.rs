//! Message deduplication node for removing duplicate messages from streams.
//!
//! This module provides [`MessageDedupe`], a graph node that deduplicates messages
//! based on message IDs or content. It filters duplicate messages based on their
//! unique identifiers, maintaining a cache of recently seen message IDs. It wraps
//! [`MessageDedupeTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`MessageDedupe`] is useful for removing duplicate messages from streams in
//! graph-based pipelines. It maintains a cache of recently seen message IDs and
//! filters out any messages whose ID has already been seen within the configured
//! window, making it ideal for ensuring message uniqueness.
//!
//! # Key Concepts
//!
//! - **Message Deduplication**: Filters duplicate messages based on message IDs
//! - **Deduplication Window**: Configurable window for determining what counts as a duplicate
//! - **ID-Based Filtering**: Uses message IDs for efficient duplicate detection
//! - **Transformer Wrapper**: Wraps `MessageDedupeTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`MessageDedupe<T>`]**: Node that deduplicates messages based on IDs
//! - **[`DeduplicationWindow`]**: Enum representing different window types (Count, Time)
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::MessageDedupe;
//! use streamweave::transformers::DeduplicationWindow;
//!
//! // Create a message dedupe node with count-based window
//! let dedupe = MessageDedupe::<i32>::new()
//!     .with_window(DeduplicationWindow::Count(1000));
//! ```
//!
//! ## With Time-Based Window
//!
//! ```rust
//! use streamweave::graph::nodes::MessageDedupe;
//! use streamweave::transformers::DeduplicationWindow;
//! use std::time::Duration;
//!
//! // Create a message dedupe node with time-based window
//! let dedupe = MessageDedupe::<String>::new()
//!     .with_window(DeduplicationWindow::Time(Duration::from_secs(60)));
//! ```
//!
//! # Design Decisions
//!
//! - **Message ID-Based**: Uses message IDs for efficient duplicate detection
//! - **Configurable Window**: Supports both count-based and time-based windows
//! - **Cache Management**: Maintains a cache of recently seen IDs for efficiency
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`MessageDedupe`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::message::Message;
use crate::transformers::{DeduplicationWindow, MessageDedupeTransformer};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that deduplicates messages based on message IDs or content.
///
/// This node wraps `MessageDedupeTransformer` for use in graphs. It maintains
/// a cache of recently seen message IDs and filters out any messages whose ID
/// has already been seen within the configured window.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{MessageDedupe, TransformerNode};
/// use crate::transformers::DeduplicationWindow;
///
/// let dedupe = MessageDedupe::new()
///     .with_window(DeduplicationWindow::Count(1000));
/// let node = TransformerNode::from_transformer(
///     "dedupe".to_string(),
///     dedupe,
/// );
/// ```
pub struct MessageDedupe<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying message dedupe transformer
  transformer: MessageDedupeTransformer<T>,
}

impl<T> MessageDedupe<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `MessageDedupe` node with default settings.
  pub fn new() -> Self {
    Self {
      transformer: MessageDedupeTransformer::new(),
    }
  }

  /// Sets the deduplication window.
  ///
  /// # Arguments
  ///
  /// * `window` - The deduplication window configuration.
  pub fn with_window(mut self, window: DeduplicationWindow) -> Self {
    self.transformer = self.transformer.with_window(window);
    self
  }

  /// Sets the error handling strategy for this node.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Message<T>>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }
}

impl<T> Default for MessageDedupe<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> Clone for MessageDedupe<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for MessageDedupe<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = Message<T>;
  type InputStream = Pin<Box<dyn Stream<Item = Message<T>> + Send>>;
}

impl<T> Output for MessageDedupe<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = Message<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Message<T>> + Send>>;
}

#[async_trait]
impl<T> Transformer for MessageDedupe<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (Message<T>,);
  type OutputPorts = (Message<T>,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Message<T>>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<Message<T>> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Message<T>> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<Message<T>>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<Message<T>>) -> ErrorContext<Message<T>> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}
