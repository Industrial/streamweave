//! Standard I/O (stdio) integration for StreamWeave
//!
//! This package provides producers and consumers for integrating StreamWeave
//! with POSIX standard streams (stdin, stdout, stderr).
//!
//! ## Producer
//!
//! The `StdinProducer` reads lines from standard input (stdin) and produces
//! them into a StreamWeave stream.
//!
//! ## Consumers
//!
//! - **`StdoutConsumer`**: Writes items from a StreamWeave stream to standard output (stdout)
//! - **`StderrConsumer`**: Writes items from a StreamWeave stream to standard error (stderr)
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave_stdio::{StdinProducer, StdoutConsumer};
//! use streamweave::Pipeline;
//!
//! let pipeline = Pipeline::new()
//!     .producer(StdinProducer::new())
//!     .transformer(/* ... */)
//!     .consumer(StdoutConsumer::new());
//! ```

pub mod consumers;
pub mod producers;

pub use consumers::*;
pub use producers::*;
