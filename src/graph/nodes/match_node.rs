//! # Match Node
//!
//! A transform node that routes data items to different outputs based on pattern matching.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration updates that define the pattern matching function
//! - **Input**: `"in"` - Receives data items to match
//! - **Output**: `"out_0"`, `"out_1"`, ..., `"out_n"` - Sends data items that match specific patterns (dynamic ports)
//! - **Output**: `"default"` - Sends data items that don't match any pattern
//! - **Output**: `"error"` - Sends errors that occur during pattern matching
//!
//! ## Configuration
//!
//! The configuration port receives `MatchConfig` (which is `Arc<dyn MatchFunction>`) that defines
//! the pattern matching logic. The function returns `Some(branch_index)` to route to `out_{branch_index}`,
//! or `None` to route to the `default` port.

use crate::graph::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::graph::nodes::common::{BaseNode, MessageType};
use async_trait::async_trait;
use futures::stream;
use regex::Regex;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Trait for asynchronous pattern matching functions used by MatchNode.
///
/// Implementations of this trait define how to match patterns on input data.
/// The function receives an `Arc<dyn Any + Send + Sync>` and returns
/// `Some(branch_index)` to route to `out_{branch_index}`, or `None` to route to `default`.
#[async_trait]
pub trait MatchFunction: Send + Sync {
  /// Matches the input value against patterns and returns the branch index.
  ///
  /// # Arguments
  ///
  /// * `value` - The input value wrapped in `Arc<dyn Any + Send + Sync>`
  ///
  /// # Returns
  ///
  /// `Ok(Some(branch_index))` if the item matches pattern at `branch_index` (routes to `out_{branch_index}`),
  /// `Ok(None)` if the item doesn't match any pattern (routes to `default`),
  /// or `Err(error_message)` if an error occurs (routes to `error`).
  async fn apply(&self, value: Arc<dyn Any + Send + Sync>) -> Result<Option<usize>, String>;
}

/// Configuration for MatchNode that defines the pattern matching function.
///
/// Contains an Arc-wrapped function that implements `MatchFunction` to perform the matching.
pub type MatchConfig = Arc<dyn MatchFunction>;

/// Wrapper type that implements MatchFunction for async closures.
struct MatchFunctionWrapper<F> {
  function: F,
}

#[async_trait::async_trait]
impl<F> MatchFunction for MatchFunctionWrapper<F>
where
  F: Fn(
      Arc<dyn Any + Send + Sync>,
    )
      -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<usize>, String>> + Send>>
    + Send
    + Sync,
{
  async fn apply(&self, value: Arc<dyn Any + Send + Sync>) -> Result<Option<usize>, String> {
    (self.function)(value).await
  }
}

/// Helper function to create a MatchConfig from an async closure.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::graph::nodes::{MatchConfig, match_config};
///
/// // Create a config that routes based on number ranges
/// let config: MatchConfig = match_config(|value| async move {
///     if let Ok(arc_i32) = value.downcast::<i32>() {
///         let n = *arc_i32;
///         if n < 0 {
///             Ok(Some(0)) // Route to out_0
///         } else if n < 10 {
///             Ok(Some(1)) // Route to out_1
///         } else {
///             Ok(None) // Route to default
///         }
///     } else {
///         Err("Expected i32".to_string())
///     }
/// });
/// ```
pub fn match_config<F, Fut>(function: F) -> MatchConfig
where
  F: Fn(Arc<dyn Any + Send + Sync>) -> Fut + Send + Sync + 'static,
  Fut: std::future::Future<Output = Result<Option<usize>, String>> + Send + 'static,
{
  Arc::new(MatchFunctionWrapper {
    function: move |v| {
      Box::pin(function(v))
        as std::pin::Pin<
          Box<dyn std::future::Future<Output = Result<Option<usize>, String>> + Send>,
        >
    },
  })
}

/// Helper function to create a MatchConfig for regex pattern matching on strings.
///
/// This helper allows matching string values against regex patterns and routing them to branches.
/// Patterns are matched in order, and the first match determines the branch.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::graph::nodes::{MatchConfig, match_regex};
/// use regex::Regex;
///
/// // Route email addresses to branch 0, URLs to branch 1, others to default
/// let patterns = vec![
///     (Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap(), 0),
///     (Regex::new(r"^https?://").unwrap(), 1),
/// ];
///
/// let config: MatchConfig = match_regex(patterns);
/// ```
pub fn match_regex(patterns: Vec<(Regex, usize)>) -> MatchConfig {
  match_config(move |value| {
    let patterns = patterns.clone();
    async move {
      // Try to downcast to String
      if let Ok(arc_str) = value.clone().downcast::<String>() {
        let s = arc_str.as_str();
        for (pattern, branch_index) in &patterns {
          if pattern.is_match(s) {
            return Ok(Some(*branch_index));
          }
        }
        Ok(None) // No match - route to default
      } else if let Ok(arc_str) = value.downcast::<&str>() {
        let s = *arc_str;
        for (pattern, branch_index) in &patterns {
          if pattern.is_match(s) {
            return Ok(Some(*branch_index));
          }
        }
        Ok(None) // No match - route to default
      } else {
        Err("Expected String or &str for regex matching".to_string())
      }
    }
  })
}

/// Helper function to create a MatchConfig for exact value matching.
///
/// This helper allows matching exact values against a list and routing them to branches.
/// Works with any type that implements `Clone`, `PartialEq`, `Send`, and `Sync`.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::graph::nodes::{MatchConfig, match_exact};
///
/// // Route "error" to branch 0, "warning" to branch 1, others to default
/// let values: Vec<(&str, usize)> = vec![
///     ("error", 0),
///     ("warning", 1),
/// ];
///
/// let config: MatchConfig = match_exact(values);
/// ```
pub fn match_exact<T>(values: Vec<(T, usize)>) -> MatchConfig
where
  T: Send + Sync + Clone + PartialEq + 'static,
{
  match_config(move |value| {
    let values = values.clone();
    async move {
      // Try to downcast to T
      if let Ok(arc_t) = value.downcast::<T>() {
        let val = (*arc_t).clone();
        for (pattern_val, branch_index) in &values {
          if &val == pattern_val {
            return Ok(Some(*branch_index));
          }
        }
        Ok(None) // No match - route to default
      } else {
        Err(format!(
          "Expected {} for exact matching",
          std::any::type_name::<T>()
        ))
      }
    }
  })
}

/// Helper function to create a MatchConfig for exact string matching.
///
/// This is a convenience function that works with String values and &str patterns.
/// It's more ergonomic than using `match_exact` with String values directly.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::graph::nodes::{MatchConfig, match_exact_string};
///
/// // Route "error" to branch 0, "warning" to branch 1, others to default
/// let patterns = vec![("error", 0), ("warning", 1)];
/// let config: MatchConfig = match_exact_string(patterns);
/// ```
pub fn match_exact_string(patterns: Vec<(&str, usize)>) -> MatchConfig {
  // Convert &str patterns to String to avoid lifetime issues
  let patterns: Vec<(String, usize)> = patterns
    .into_iter()
    .map(|(s, idx)| (s.to_string(), idx))
    .collect();
  match_config(move |value| {
    let patterns = patterns.clone();
    async move {
      // Try to downcast to String
      if let Ok(arc_string) = value.downcast::<String>() {
        let s = arc_string.as_str();
        for (pattern, branch_index) in &patterns {
          if s == pattern {
            return Ok(Some(*branch_index));
          }
        }
        Ok(None) // No match - route to default
      } else {
        Err("Expected String for exact string matching".to_string())
      }
    }
  })
}

/// Helper function to create a MatchConfig for numeric range matching.
///
/// This helper allows matching numeric values against ranges and routing them to branches.
/// The matcher function receives the numeric value and returns the branch index.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::graph::nodes::{MatchConfig, match_numeric_range};
///
/// // Route negative numbers to branch 0, 0-10 to branch 1, 10+ to default
/// let config: MatchConfig = match_numeric_range(|n: i32| async move {
///     if n < 0 {
///         Ok(Some(0))
///     } else if n < 10 {
///         Ok(Some(1))
///     } else {
///         Ok(None) // Route to default
///     }
/// });
/// ```
pub fn match_numeric_range<F, Fut>(matcher: F) -> MatchConfig
where
  F: Fn(i32) -> Fut + Send + Sync + Clone + 'static,
  Fut: std::future::Future<Output = Result<Option<usize>, String>> + Send + 'static,
{
  match_config(move |value| {
    let matcher = matcher.clone();
    async move {
      // Try common numeric types and convert to i32
      if let Ok(arc_i32) = value.clone().downcast::<i32>() {
        return matcher(*arc_i32).await;
      }
      if let Ok(arc_i64) = value.clone().downcast::<i64>() {
        return matcher(*arc_i64 as i32).await;
      }
      if let Ok(arc_u32) = value.clone().downcast::<u32>() {
        return matcher(*arc_u32 as i32).await;
      }
      if let Ok(arc_u64) = value.clone().downcast::<u64>() {
        return matcher(*arc_u64 as i32).await;
      }
      if let Ok(arc_f32) = value.clone().downcast::<f32>() {
        return matcher(*arc_f32 as i32).await;
      }
      if let Ok(arc_f64) = value.clone().downcast::<f64>() {
        return matcher(*arc_f64 as i32).await;
      }
      Err("Expected numeric type (i32, i64, u32, u64, f32, f64) for range matching".to_string())
    }
  })
}

/// A node that routes data items to different outputs based on pattern matching.
///
/// The node receives configuration that defines pattern matching logic, and routes
/// each input item to the appropriate output port based on the match result.
pub struct MatchNode {
  pub(crate) base: BaseNode,
  current_config: Arc<Mutex<Option<MatchConfig>>>,
  max_branches: usize,
}

impl MatchNode {
  /// Creates a new MatchNode with the given name and maximum number of branches.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node
  /// * `max_branches` - Maximum number of pattern branches (creates `out_0` through `out_{max_branches-1}`)
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::nodes::MatchNode;
  ///
  /// let node = MatchNode::new("matcher".to_string(), 3);
  /// // Creates ports: configuration, in → out_0, out_1, out_2, default, error
  /// ```
  pub fn new(name: String, max_branches: usize) -> Self {
    let mut output_ports = vec!["default".to_string(), "error".to_string()];
    for i in 0..max_branches {
      output_ports.push(format!("out_{}", i));
    }

    Self {
      base: BaseNode::new(
        name,
        vec!["configuration".to_string(), "in".to_string()],
        output_ports,
      ),
      current_config: Arc::new(Mutex::new(None)),
      max_branches,
    }
  }

  /// Returns whether the node has a configuration set.
  pub fn has_config(&self) -> bool {
    self
      .current_config
      .try_lock()
      .map(|g| g.is_some())
      .unwrap_or(false)
  }

  /// Returns the maximum number of branches this node supports.
  pub fn max_branches(&self) -> usize {
    self.max_branches
  }
}

#[async_trait]
impl Node for MatchNode {
  fn name(&self) -> &str {
    self.base.name()
  }

  fn set_name(&mut self, name: &str) {
    self.base.set_name(name);
  }

  fn input_port_names(&self) -> &[String] {
    self.base.input_port_names()
  }

  fn output_port_names(&self) -> &[String] {
    self.base.output_port_names()
  }

  fn has_input_port(&self, name: &str) -> bool {
    self.base.has_input_port(name)
  }

  fn has_output_port(&self, name: &str) -> bool {
    self.base.has_output_port(name)
  }

  fn execute(
    &self,
    mut inputs: InputStreams,
  ) -> Pin<
    Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>,
  > {
    let config_state = Arc::clone(&self.current_config);
    let max_branches = self.max_branches;

    Box::pin(async move {
      // Extract input streams
      let config_stream = inputs
        .remove("configuration")
        .ok_or("Missing 'configuration' input")?;
      let data_stream = inputs.remove("in").ok_or("Missing 'in' input")?;

      // Tag streams to distinguish config from data
      let config_stream = config_stream.map(|item| (MessageType::Config, item));
      let data_stream = data_stream.map(|item| (MessageType::Data, item));

      // Merge streams
      let merged_stream = stream::select(config_stream, data_stream);

      // Create output channels for all branches plus default and error
      type ChannelPair = (
        tokio::sync::mpsc::Sender<Arc<dyn Any + Send + Sync>>,
        tokio::sync::mpsc::Receiver<Arc<dyn Any + Send + Sync>>,
      );
      let mut output_channels: HashMap<String, ChannelPair> = HashMap::new();

      // Create channels for each branch
      for i in 0..max_branches {
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        output_channels.insert(format!("out_{}", i), (tx, rx));
      }

      // Create channels for default and error
      let (default_tx, default_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);
      output_channels.insert("default".to_string(), (default_tx.clone(), default_rx));
      output_channels.insert("error".to_string(), (error_tx.clone(), error_rx));

      // Clone senders for the processing task
      let mut branch_txs: HashMap<String, tokio::sync::mpsc::Sender<Arc<dyn Any + Send + Sync>>> =
        HashMap::new();
      for (port, (tx, _)) in &output_channels {
        if port != "error" {
          branch_txs.insert(port.clone(), tx.clone());
        }
      }
      let error_tx_clone = error_tx.clone();

      // Process the merged stream
      let config_state_clone = Arc::clone(&config_state);

      tokio::spawn(async move {
        let mut merged = merged_stream;
        let mut current_config: Option<MatchConfig> = None;

        while let Some((msg_type, item)) = merged.next().await {
          match msg_type {
            MessageType::Config => {
              // Update configuration - handle both Arc<Arc<dyn MatchFunction>> and Arc<dyn MatchFunction>
              if let Ok(arc_arc_fn) = item.clone().downcast::<Arc<Arc<dyn MatchFunction>>>() {
                let cfg = Arc::clone(&**arc_arc_fn);
                current_config = Some(Arc::clone(&cfg));
                *config_state_clone.lock().await = Some(cfg);
              } else if let Ok(arc_function) = item.clone().downcast::<Arc<dyn MatchFunction>>() {
                let cfg = Arc::clone(&*arc_function);
                current_config = Some(Arc::clone(&cfg));
                *config_state_clone.lock().await = Some(cfg);
              } else {
                let error_msg: String =
                  "Invalid configuration type - expected MatchConfig (Arc<dyn MatchFunction>)"
                    .to_string();
                let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                let _ = error_tx_clone.send(error_arc).await;
              }
            }
            MessageType::Data => {
              match &current_config {
                Some(cfg) => {
                  // Zero-copy: clone Arc reference before calling apply
                  let item_clone = item.clone();
                  match cfg.apply(item).await {
                    Ok(Some(branch_index)) => {
                      // Route to the specified branch
                      let port_name = format!("out_{}", branch_index);
                      if let Some(tx) = branch_txs.get(&port_name) {
                        let _ = tx.send(item_clone).await;
                      } else {
                        // Branch index out of range - route to default
                        if let Some(tx) = branch_txs.get("default") {
                          let _ = tx.send(item_clone).await;
                        }
                      }
                    }
                    Ok(None) => {
                      // No match - route to default
                      if let Some(tx) = branch_txs.get("default") {
                        let _ = tx.send(item_clone).await;
                      }
                    }
                    Err(error_msg) => {
                      let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                      let _ = error_tx_clone.send(error_arc).await;
                    }
                  }
                }
                None => {
                  let error_msg: String =
                    "No configuration set. Please send configuration before data.".to_string();
                  let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                  let _ = error_tx_clone.send(error_arc).await;
                }
              }
            }
          }
        }
      });

      // Convert channels to streams
      let mut outputs = HashMap::new();
      for (port, (_, rx)) in output_channels {
        outputs.insert(
          port.clone(),
          Box::pin(ReceiverStream::new(rx))
            as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
        );
      }

      Ok(outputs)
    })
  }
}
