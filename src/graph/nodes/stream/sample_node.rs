//! # Sample Node
//!
//! A transform node that samples items from the input stream based on a rate.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items
//! - **Input**: `"rate"` - Receives rate value (numeric: sample every Nth item, where rate=N)
//! - **Output**: `"out"` - Sends sampled items
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node samples items from the input stream based on the rate.
//! The rate determines how often items are sampled:
//! - rate=1: Forward every item (no sampling)
//! - rate=2: Forward every 2nd item
//! - rate=3: Forward every 3rd item
//! - etc.
//! It supports:
//! - Sampling based on a numeric rate (usize, i32, i64, u32, u64)
//! - Error handling: Invalid rate values (zero, negative, wrong type) result in errors sent to the error port

use crate::graph::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::graph::nodes::common::BaseNode;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper function to extract usize from various numeric types.
///
/// Accepts:
/// - usize directly
/// - Numeric types (i32, i64, u32, u64) converted to usize
fn get_usize(value: &Arc<dyn Any + Send + Sync>) -> Result<usize, String> {
  if let Ok(arc_usize) = value.clone().downcast::<usize>() {
    Ok(*arc_usize)
  } else if let Ok(arc_i32) = value.clone().downcast::<i32>() {
    if *arc_i32 <= 0 {
      return Err("Rate must be positive".to_string());
    }
    (*arc_i32)
      .try_into()
      .map_err(|_| "Rate too large".to_string())
  } else if let Ok(arc_i64) = value.clone().downcast::<i64>() {
    if *arc_i64 <= 0 {
      return Err("Rate must be positive".to_string());
    }
    (*arc_i64)
      .try_into()
      .map_err(|_| "Rate too large".to_string())
  } else if let Ok(arc_u32) = value.clone().downcast::<u32>() {
    if *arc_u32 == 0 {
      return Err("Rate must be positive".to_string());
    }
    (*arc_u32)
      .try_into()
      .map_err(|_| "Rate too large".to_string())
  } else if let Ok(arc_u64) = value.clone().downcast::<u64>() {
    if *arc_u64 == 0 {
      return Err("Rate must be positive".to_string());
    }
    (*arc_u64)
      .try_into()
      .map_err(|_| "Rate too large".to_string())
  } else {
    Err(format!(
      "Unsupported type for rate: {} (must be numeric)",
      std::any::type_name_of_val(&**value)
    ))
  }
}

/// Enum to tag messages from different input ports for merging.
#[derive(Debug, PartialEq)]
enum InputPort {
  Config,
  In,
  Rate,
}

/// A node that samples items from the input stream based on a rate.
///
/// The rate is received on the "rate" port and can be:
/// - A numeric value (usize, i32, i64, u32, u64) representing "every Nth item"
/// - rate=1: Forward every item
/// - rate=2: Forward every 2nd item
/// - rate=N: Forward every Nth item
/// The node forwards items from the "in" port to the "out" port based on the sampling rate.
pub struct SampleNode {
  pub(crate) base: BaseNode,
}

impl SampleNode {
  /// Creates a new SampleNode with the given name.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  ///
  /// # Returns
  ///
  /// A new `SampleNode` instance.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::graph::nodes::stream::SampleNode;
  ///
  /// let node = SampleNode::new("sample".to_string());
  /// // Creates ports: configuration, in, rate → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "rate".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
#[allow(clippy::type_complexity)]
impl Node for SampleNode {
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
    Box::pin(async move {
      // Extract input streams
      let _config_stream = inputs.remove("configuration");
      let in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
      let rate_stream = inputs
        .remove("rate")
        .ok_or("Missing 'rate' input")?;

      // Tag streams to distinguish inputs
      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let rate_stream = rate_stream.map(|item| (InputPort::Rate, item));

      // Merge streams
      let merged_stream: Pin<
        Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
      > = Box::pin(stream::select_all(vec![
        Box::pin(in_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
        Box::pin(rate_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
      ]));

      // Create output channels
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged_stream = merged_stream;
        let mut rate: Option<usize> = None;
        let mut item_counter: usize = 0;

        while let Some((port, item)) = merged_stream.next().await {
          match port {
            InputPort::Config => {
              // Configuration port is unused for now
            }
            InputPort::Rate => {
              // Update rate
              match get_usize(&item) {
                Ok(r) => {
                  rate = Some(r);
                }
                Err(e) => {
                  let error_arc = Arc::new(e) as Arc<dyn Any + Send + Sync>;
                  let _ = error_tx_clone.send(error_arc).await;
                }
              }
            }
            InputPort::In => {
              // Sample item based on rate
              if let Some(r) = rate {
                item_counter += 1;
                // Forward item if it matches the sampling rate (every Nth item)
                if item_counter % r == 0 {
                  let _ = out_tx_clone.send(item).await;
                }
              } else {
                // No rate set yet - wait for rate before processing items
                // Items arriving before rate will be dropped
              }
            }
          }
        }
      });

      // Convert channels to streams
      let mut outputs = HashMap::new();
      outputs.insert(
        "out".to_string(),
        Box::pin(ReceiverStream::new(out_rx))
          as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );
      outputs.insert(
        "error".to_string(),
        Box::pin(ReceiverStream::new(error_rx))
          as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );

      Ok(outputs)
    })
  }
}

