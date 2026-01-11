//! Output trait for components that produce output streams.
//!
//! This module defines the [`Output`] trait for components that produce output streams
//! in the StreamWeave framework. The `Output` trait is implemented by producers and
//! transformers that generate data for downstream components.
//!
//! # Overview
//!
//! The [`Output`] trait defines the interface for components that produce data streams.
//! It is implemented by:
//!
//! - **Producers**: Components that generate streams (start of pipeline)
//! - **Transformers**: Components that transform input streams into output streams
//!
//! # Key Concepts
//!
//! - **Output Type**: `Output::Output` is `Message<T>` where `T` is the payload type
//! - **OutputStream**: A pinned, boxed async stream yielding `Message<T>` items
//! - **Type Safety**: All output types must be `Send` for cross-thread usage
//! - **Universal Message Model**: All items flowing through StreamWeave are wrapped in messages
//!
//! # Core Types
//!
//! - **[`Output`]**: Trait for components that produce output streams
//!
//! # Quick Start
//!
//! ## Implementing the Output Trait
//!
//! ```rust
//! use crate::output::Output;
//! use crate::message::Message;
//! use futures::Stream;
//! use std::pin::Pin;
//!
//! struct MyProducer;
//!
//! impl Output for MyProducer {
//!     type Output = Message<i32>;
//!     type OutputStream = Pin<Box<dyn Stream<Item = Message<i32>> + Send>>;
//! }
//! ```
//!
//! # Design Decisions
//!
//! - **Message-Based**: All output is `Message<T>` to support the universal message model
//! - **Async Streams**: Uses async streams for efficient, non-blocking I/O
//! - **Pinned Boxed Streams**: Uses `Pin<Box<dyn Stream + Send>>` for trait object streams
//! - **Send Bound**: Requires `Send` for cross-thread safety
//! - **Trait-Based Design**: Trait-based design enables generic pipeline construction
//!
//! # Integration with StreamWeave
//!
//! The [`Output`] trait is used internally by the pipeline and graph systems to ensure
//! type-safe stream connections. It works together with the [`Input`] trait to create
//! type-safe pipelines where output types must match input types.
//!
//! Users typically interact with concrete implementations (producers, transformers) rather
//! than the trait directly, but understanding the trait is important for implementing
//! custom components.

// Import for rustdoc links
#[allow(unused_imports)]
use crate::input::Input;

use futures::Stream;

/// Trait for components that can produce output streams.
///
/// This trait defines the interface for components that generate data streams.
/// It is implemented by producers and transformers that output data.
///
/// **Note**: In StreamWeave, `Output::Output` is `Message<T>` where `T` is the payload type.
/// All items flowing through StreamWeave are wrapped in messages.
pub trait Output
where
  Self::Output: Send + 'static,
{
  /// The type of items produced by this output stream.
  /// This is `Message<T>` where `T` is the payload type.
  type Output;
  /// The output stream type that yields items of type `Self::Output`.
  /// This yields `Message<T>` items.
  type OutputStream: Stream<Item = Self::Output> + Send + 'static;
}
