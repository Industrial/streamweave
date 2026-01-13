//! # Signal Producer
//!
//! Producer for Unix signal handling in StreamWeave pipelines.
//!
//! This module provides [`SignalProducer`], a producer that emits events when Unix signals
//! are received. This enables graceful shutdown handling, signal-based triggering, and
//! integration with Unix signal management systems.
//!
//! # Overview
//!
//! [`SignalProducer`] listens for Unix signals (SIGINT, SIGTERM) and emits them as items
//! in a stream. This allows StreamWeave pipelines to respond to signals for graceful
//! shutdown, conditional processing, or event-driven workflows.
//!
//! # Key Concepts
//!
//! - **Signal Types**: Supports common Unix signals (SIGINT, SIGTERM)
//! - **Async Signal Handling**: Uses Tokio's signal handling for async/await compatibility
//! - **Continuous Stream**: Produces signals continuously until the producer is dropped
//! - **Unix-Only**: Only available on Unix-like systems (Linux, macOS, etc.)
//!
//! # Core Types
//!
//! - **[`SignalProducer`]**: Producer that emits Unix signals as stream items
//! - **[`Signal`]**: Enum representing supported Unix signals
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust,no_run
//! use streamweave::producers::SignalProducer;
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a signal producer
//! let mut producer = SignalProducer::new();
//!
//! // Generate the stream
//! let mut stream = producer.produce();
//!
//! // Listen for signals
//! while let Some(signal) = stream.next().await {
//!     match signal {
//!         streamweave::producers::Signal::Interrupt => {
//!             println!("Received SIGINT (Ctrl+C)");
//!             // Handle interrupt signal
//!         }
//!         streamweave::producers::Signal::Terminate => {
//!             println!("Received SIGTERM");
//!             // Handle termination signal
//!             break; // Exit on termination
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Graceful Shutdown Pattern
//!
//! ```rust,no_run
//! use streamweave::producers::SignalProducer;
//! use futures::StreamExt;
//! use tokio::select;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut producer = SignalProducer::new();
//! let mut signal_stream = producer.produce();
//!
//! // Process other work while listening for signals
//! loop {
//!     select! {
//!         signal = signal_stream.next() => {
//!             if let Some(signal) = signal {
//!                 match signal {
//!                     streamweave::producers::Signal::Interrupt |
//!                     streamweave::producers::Signal::Terminate => {
//!                         println!("Shutting down gracefully...");
//!                         break;
//!                     }
//!                 }
//!             }
//!         }
//!         // Other async work...
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Signal Enum Usage
//!
//! ```rust
//! use streamweave::producers::Signal;
//!
//! // Create from signal number
//! let signal = Signal::from_number(2); // SIGINT
//! assert_eq!(signal, Some(Signal::Interrupt));
//!
//! // Get signal number
//! let sigint = Signal::Interrupt;
//! assert_eq!(sigint.number(), 2);
//!
//! let sigterm = Signal::Terminate;
//! assert_eq!(sigterm.number(), 15);
//! ```
//!
//! # Design Decisions
//!
//! ## Unix-Only Design
//!
//! This producer is Unix-specific and uses Tokio's Unix signal handling. On non-Unix
//! systems, this producer will not compile or will fail at runtime. This design decision
//! aligns with the Unix signal model and ensures compatibility with Unix-based systems.
//!
//! ## Continuous Stream
//!
//! The producer continuously listens for signals and emits them as stream items. The stream
//! will continue until the producer is dropped, allowing for long-running signal handlers.
//! This design enables graceful shutdown patterns and signal-driven workflows.
//!
//! ## Signal Selection
//!
//! Currently supports SIGINT and SIGTERM, which are the most common signals for graceful
//! shutdown. Additional signals can be added by extending the [`Signal`] enum and the
//! producer implementation.
//!
//! ## Async Signal Handling
//!
//! Uses Tokio's async signal handling to integrate seamlessly with async/await code.
//! This avoids blocking the async runtime and allows signals to be handled concurrently
//! with other async operations.
//!
//! # Integration with StreamWeave
//!
//! [`SignalProducer`] integrates seamlessly with StreamWeave's pipeline and graph systems:
//!
//! - **Pipeline API**: Use in pipelines for signal-driven workflows
//! - **Graph API**: Wrap in graph nodes for graph-based signal handling
//! - **Error Handling**: Supports standard error handling strategies
//! - **Configuration**: Supports configuration via [`ProducerConfig`]
//!
//! # Common Patterns
//!
//! ## Graceful Shutdown
//!
//! Use signals to trigger graceful shutdown of pipelines:
//!
//! ```rust,no_run
//! use streamweave::producers::SignalProducer;
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut producer = SignalProducer::new()
//!     .with_name("signal-handler".to_string());
//!
//! let mut stream = producer.produce();
//! while let Some(signal) = stream.next().await {
//!     // Trigger shutdown on any signal
//!     println!("Received signal: {:?}, shutting down...", signal);
//!     break;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Conditional Processing
//!
//! Use signals to trigger conditional processing or state changes:
//!
//! ```rust,no_run
//! use streamweave::producers::SignalProducer;
//! use futures::StreamExt;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut producer = SignalProducer::new();
//! let mut stream = producer.produce();
//!
//! while let Some(signal) = stream.next().await {
//!     match signal {
//!         streamweave::producers::Signal::Interrupt => {
//!             // Pause processing on SIGINT
//!             println!("Pausing...");
//!         }
//!         streamweave::producers::Signal::Terminate => {
//!             // Stop processing on SIGTERM
//!             println!("Stopping...");
//!             break;
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Platform Notes
//!
//! - **Unix Only**: This producer only works on Unix-like systems (Linux, macOS, BSD, etc.)
//! - **Windows**: Not supported on Windows (use alternative mechanisms like Ctrl+C handlers)
//! - **Signal Numbers**: Signal numbers are system-specific but SIGINT (2) and SIGTERM (15) are standard
//! - **Signal Delivery**: Signals are delivered asynchronously and may be coalesced

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Output, Producer, ProducerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Represents a Unix signal that was received.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
  /// SIGINT - Interrupt signal
  Interrupt,
  /// SIGTERM - Termination signal
  Terminate,
}

impl Signal {
  /// Convert from a signal number
  pub fn from_number(sig: i32) -> Option<Self> {
    match sig {
      2 => Some(Signal::Interrupt),  // SIGINT
      15 => Some(Signal::Terminate), // SIGTERM
      _ => None,
    }
  }

  /// Get the signal number
  pub fn number(&self) -> i32 {
    match self {
      Signal::Interrupt => 2,
      Signal::Terminate => 15,
    }
  }
}

/// A producer that emits events when Unix signals are received.
#[derive(Clone)]
pub struct SignalProducer {
  /// Configuration for the producer
  pub config: ProducerConfig<Signal>,
}

impl SignalProducer {
  /// Creates a new `SignalProducer`.
  pub fn new() -> Self {
    Self {
      config: ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Signal>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Default for SignalProducer {
  fn default() -> Self {
    Self::new()
  }
}

impl Output for SignalProducer {
  type Output = Signal;
  type OutputStream = Pin<Box<dyn Stream<Item = Signal> + Send>>;
}

#[async_trait]
impl Producer for SignalProducer {
  type OutputPorts = (Signal,);

  fn produce(&mut self) -> Self::OutputStream {
    Box::pin(async_stream::stream! {
      // Create signal streams for common signals
      if let Ok(mut sigint) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
        && let Ok(mut sigterm) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
      {
        loop {
          tokio::select! {
            _ = sigint.recv() => {
              yield Signal::Interrupt;
            }
            _ = sigterm.recv() => {
              yield Signal::Terminate;
            }
          }
        }
      }
    })
  }

  fn set_config_impl(&mut self, config: ProducerConfig<Signal>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<Signal> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<Signal> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Signal>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Signal>) -> ErrorContext<Signal> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "signal_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "signal_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
