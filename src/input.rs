//! Input trait for components that consume input streams.
//!
//! This module defines the [`Input`] trait for components that consume input streams
//! in the StreamWeave framework. The `Input` trait is implemented by transformers and
//! consumers that receive data from upstream components.
//!
//! # Overview
//!
//! The [`Input`] trait defines the interface for components that consume data streams.
//! It is implemented by:
//!
//! - **Transformers**: Components that transform input streams into output streams
//! - **Consumers**: Components that consume streams (end of pipeline)
//!
//! # Key Concepts
//!
//! - **Input Type**: `Input::Input` is `Message<T>` where `T` is the payload type
//! - **InputStream**: A pinned, boxed async stream yielding `Message<T>` items
//! - **Type Safety**: All input types must be `Send` for cross-thread usage
//! - **Universal Message Model**: All items flowing through StreamWeave are wrapped in messages
//!
//! # Core Types
//!
//! - **[`Input`]**: Trait for components that consume input streams
//!
//! # Quick Start
//!
//! ## Implementing the Input Trait
//!
//! ```rust
//! use crate::input::Input;
//! use crate::message::Message;
//! use futures::Stream;
//! use std::pin::Pin;
//!
//! struct MyTransformer;
//!
//! impl Input for MyTransformer {
//!     type Input = Message<i32>;
//!     type InputStream = Pin<Box<dyn Stream<Item = Message<i32>> + Send>>;
//! }
//! ```
//!
//! # Design Decisions
//!
//! - **Message-Based**: All input is `Message<T>` to support the universal message model
//! - **Async Streams**: Uses async streams for efficient, non-blocking I/O
//! - **Pinned Boxed Streams**: Uses `Pin<Box<dyn Stream + Send>>` for trait object streams
//! - **Send Bound**: Requires `Send` for cross-thread safety
//! - **Trait-Based Design**: Trait-based design enables generic pipeline construction
//!
//! # Integration with StreamWeave
//!
//! The [`Input`] trait is used internally by the pipeline and graph systems to ensure
//! type-safe stream connections. It works together with the [`crate::Output`] trait to create
//! type-safe pipelines where output types must match input types.
//!
//! Users typically interact with concrete implementations (transformers, consumers) rather
//! than the trait directly, but understanding the trait is important for implementing
//! custom components.

use futures::Stream;
// Import for rustdoc link
#[allow(unused_imports)]
use crate::output::Output;

/// Trait for components that can provide input streams.
///
/// This trait defines the interface for components that produce data streams.
/// It is implemented by transformers and consumers that receive data.
///
/// **Note**: In StreamWeave, `Input::Input` is `Message<T>` where `T` is the payload type.
/// All items flowing through StreamWeave are wrapped in messages.
pub trait Input
where
  Self::Input: Send + 'static,
{
  /// The type of items produced by this input stream.
  /// This is `Message<T>` where `T` is the payload type.
  type Input;
  /// The input stream type that yields items of type `Self::Input`.
  /// This yields `Message<T>` items.
  type InputStream: Stream<Item = Self::Input> + Send + 'static;
}
