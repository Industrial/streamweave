//! Match router for routing items based on pattern matching.
//!
//! This module provides [`Match`] and [`Pattern`], types for routing items based on
//! pattern matching (match/switch pattern) in graph-based pipelines. It supports
//! multiple patterns and an optional default port, making it ideal for conditional
//! routing based on complex matching conditions. It implements [`OutputRouter`]
//! for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`Match`] is useful for routing items based on pattern matching in graph-based
//! pipelines. It supports multiple patterns with an optional default port, allowing
//! complex conditional routing scenarios similar to switch/match statements in
//! programming languages.
//!
//! # Key Concepts
//!
//! - **Pattern Matching**: Routes items based on pattern matching against multiple
//!   patterns
//! - **Multiple Patterns**: Supports multiple patterns with first-match semantics
//! - **Default Port**: Optional default port for items that don't match any pattern
//! - **Pattern Trait**: Uses the `Pattern` trait for flexible pattern matching
//!
//! # Core Types
//!
//! - **[`Match<O>`]**: Router that routes items based on pattern matching
//! - **[`Pattern<T>`]**: Trait for pattern matching implementations
//! - **[`PredicatePattern`]**: Pattern that matches items based on a predicate
//! - **[`RangePattern`]**: Pattern that matches items within a range
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::{Match, Pattern, RangePattern};
//!
//! // Create patterns for routing
//! let patterns: Vec<Box<dyn Pattern<i32>>> = vec![
//!     Box::new(RangePattern::new(0..50, 0)),    // Route 0-49 to port 0
//!     Box::new(RangePattern::new(50..100, 1)),  // Route 50-99 to port 1
//! ];
//!
//! // Create match router with default port
//! let match_router = Match::new(patterns, Some(2));  // Default to port 2
//! ```
//!
//! ## With Predicate Patterns
//!
//! ```rust
//! use streamweave::graph::nodes::{Match, Pattern, PredicatePattern};
//!
//! // Create predicate patterns
//! let patterns: Vec<Box<dyn Pattern<String>>> = vec![
//!     Box::new(PredicatePattern::new(|s: &String| s.starts_with("A"), 0)),
//!     Box::new(PredicatePattern::new(|s: &String| s.starts_with("B"), 1)),
//! ];
//!
//! let match_router = Match::new(patterns, None);
//! ```
//!
//! # Design Decisions
//!
//! - **Pattern Trait**: Uses a trait-based approach for flexible pattern matching
//! - **First-Match Semantics**: Patterns are checked in order, first match wins
//! - **Default Port**: Optional default port for unmatched items
//! - **Router Trait**: Implements `OutputRouter` for integration with graph system
//!
//! # Integration with StreamWeave
//!
//! [`Match`] implements the [`OutputRouter`] trait and can be used in any StreamWeave
//! graph. It routes items to different output ports based on pattern matching,
//! enabling complex conditional routing patterns.

use crate::graph::router::OutputRouter;
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Pattern for matching items in a `Match` router.
///
/// This trait allows different pattern matching strategies to be used
/// with the `Match` router.
pub trait Pattern<T>: Send + Sync {
  /// Returns the port index if the pattern matches the item, `None` otherwise.
  ///
  /// # Arguments
  ///
  /// * `item` - The item to match against
  ///
  /// # Returns
  ///
  /// `Some(port_index)` if the pattern matches, `None` otherwise.
  fn matches(&self, item: &T) -> Option<usize>;
}

/// Router that routes items based on pattern matching (match/switch).
///
/// Routes items to different output ports based on pattern matching.
/// Supports multiple patterns and an optional default port.
///
/// # Example
///
/// ```rust
/// use crate::graph::control_flow::{Match, Pattern, RangePattern};
///
/// let patterns: Vec<Box<dyn Pattern<i32>>> = vec![
///     Box::new(RangePattern::new(0..50, 0)),
///     Box::new(RangePattern::new(50..100, 1)),
/// ];
/// let match_router = Match::new(patterns, Some(2)); // default to port 2
/// ```
pub struct Match<O> {
  /// Patterns to match against
  patterns: Vec<Box<dyn Pattern<O>>>,
  /// Default port index (used when no pattern matches)
  default_port: Option<usize>,
}

impl<O> Match<O>
where
  O: Send + Sync + 'static,
{
  /// Creates a new `Match` router with patterns and optional default port.
  ///
  /// # Arguments
  ///
  /// * `patterns` - Vector of patterns to match against. Patterns are checked
  ///   in order, and the first matching pattern determines the output port.
  /// * `default_port` - Optional port index to use when no pattern matches.
  ///   If `None`, items that don't match any pattern are dropped.
  ///
  /// # Returns
  ///
  /// A new `Match` router instance.
  pub fn new(patterns: Vec<Box<dyn Pattern<O>>>, default_port: Option<usize>) -> Self {
    Self {
      patterns,
      default_port,
    }
  }
}

#[async_trait]
impl<O> OutputRouter<O> for Match<O>
where
  O: Send + Sync + Clone + 'static,
{
  async fn route_stream(
    &mut self,
    stream: Pin<Box<dyn Stream<Item = O> + Send>>,
  ) -> Vec<(String, Pin<Box<dyn Stream<Item = O> + Send>>)> {
    // Calculate number of output ports
    let num_patterns = self.patterns.len();
    let num_ports = num_patterns + self.default_port.is_some() as usize;

    if num_ports == 0 {
      return Vec::new();
    }

    // Create channels for each output port
    let mut senders = Vec::new();
    let mut receivers = Vec::new();

    for _ in 0..num_ports {
      let (tx, rx) = mpsc::channel(16);
      senders.push(tx);
      receivers.push(rx);
    }

    // Move patterns and default_port into the task
    // Note: We can't clone Box<dyn Pattern>, so we move them
    // This means the router can only be used once, which is fine for stream routing
    let patterns = std::mem::take(&mut self.patterns);
    let default_port = self.default_port;
    let mut input_stream = stream;

    // Spawn routing task - zero-copy: items are moved to the matched port
    tokio::spawn(async move {
      while let Some(item) = input_stream.next().await {
        // Determine target port first
        let target_port = patterns
          .iter()
          .find_map(|pattern| pattern.matches(&item))
          .filter(|&port_idx| port_idx < senders.len())
          .or(default_port.filter(|&port| port < senders.len()));

        // Send to target port if found, otherwise drop item
        if let Some(port) = target_port {
          let _ = senders[port].send(item).await;
        }
        // Otherwise, item is dropped (no matching pattern and no default)
      }
    });

    // Create streams from receivers
    let mut output_streams: Vec<(String, Pin<Box<dyn Stream<Item = O> + Send>>)> = Vec::new();
    for (port_idx, mut rx) in receivers.into_iter().enumerate() {
      let port_name = if port_idx == 0 {
        "out".to_string()
      } else {
        format!("out_{}", port_idx)
      };
      let stream: Pin<Box<dyn Stream<Item = O> + Send>> = Box::pin(async_stream::stream! {
        while let Some(item) = rx.recv().await {
          yield item;
        }
      });
      output_streams.push((port_name, stream));
    }

    output_streams
  }

  fn output_port_names(&self) -> Vec<String> {
    // Calculate ports based on current state
    // Note: After route_stream is called, patterns are moved, so this may return incorrect results
    // In practice, output_port_names should be called before route_stream
    let num_patterns = self.patterns.len();
    let num_ports = num_patterns + self.default_port.is_some() as usize;
    (0..num_ports)
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

/// Pattern that matches items within a numeric range.
///
/// # Example
///
/// ```rust
/// use crate::graph::control_flow::{Pattern, RangePattern};
///
/// let pattern = RangePattern::new(0..100, 0);
/// assert_eq!(pattern.matches(&50), Some(0));
/// assert_eq!(pattern.matches(&150), None);
/// ```
pub struct RangePattern<T> {
  /// The range to match against
  range: std::ops::Range<T>,
  /// Port index to route to when pattern matches
  port: usize,
}

impl<T> RangePattern<T>
where
  T: PartialOrd,
{
  /// Creates a new `RangePattern`.
  ///
  /// # Arguments
  ///
  /// * `range` - The range to match against
  /// * `port` - Port index to route to when pattern matches
  ///
  /// # Returns
  ///
  /// A new `RangePattern` instance.
  pub fn new(range: std::ops::Range<T>, port: usize) -> Self {
    Self { range, port }
  }
}

impl<T> Pattern<T> for RangePattern<T>
where
  T: PartialOrd + Send + Sync,
{
  fn matches(&self, item: &T) -> Option<usize> {
    if item >= &self.range.start && item < &self.range.end {
      Some(self.port)
    } else {
      None
    }
  }
}

/// Pattern that matches items using a custom predicate function.
///
/// # Example
///
/// ```rust
/// use crate::graph::control_flow::{Pattern, PredicatePattern};
///
/// let pattern = PredicatePattern::new(|x: &i32| *x > 100, 0);
/// assert_eq!(pattern.matches(&150), Some(0));
/// assert_eq!(pattern.matches(&50), None);
/// ```
pub struct PredicatePattern<T> {
  /// Predicate function
  predicate: Arc<dyn Fn(&T) -> bool + Send + Sync>,
  /// Port index to route to when pattern matches
  port: usize,
}

impl<T> PredicatePattern<T>
where
  T: Send + Sync + 'static,
{
  /// Creates a new `PredicatePattern`.
  ///
  /// # Arguments
  ///
  /// * `predicate` - Function that returns `true` if the pattern matches
  /// * `port` - Port index to route to when pattern matches
  ///
  /// # Returns
  ///
  /// A new `PredicatePattern` instance.
  pub fn new<F>(predicate: F, port: usize) -> Self
  where
    F: Fn(&T) -> bool + Send + Sync + 'static,
  {
    Self {
      predicate: Arc::new(predicate),
      port,
    }
  }
}

impl<T> Pattern<T> for PredicatePattern<T>
where
  T: Send + Sync,
{
  fn matches(&self, item: &T) -> Option<usize> {
    if (self.predicate)(item) {
      Some(self.port)
    } else {
      None
    }
  }
}
