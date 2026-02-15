//! # Tumbling Processing-Time Window Node
//!
//! A transform node that collects items into fixed-duration windows based on
//! wall-clock (processing) time and emits them as batches.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items to window
//! - **Output**: `"out"` - Sends windows as `Vec<Arc<dyn Any + Send + Sync>>`
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Behavior
//!
//! The node buffers incoming items and emits a window every `window_size`
//! wall-clock time. Each emitted window is a vector of all items received
//! during that interval. No watermark needed; windows are closed by timer only.
//!
//! See [docs/windowing.md](../../../docs/windowing.md) §5.4.

#![allow(clippy::type_complexity)]

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// A node that emits tumbling windows every N ms of processing (wall-clock) time.
///
/// Buffers items; on each timer tick, emits the current buffer and clears it.
/// Determinism: per run only if scheduling is fixed.
pub struct TumblingProcessingTimeWindowNode {
  /// Base node functionality.
  pub(crate) base: BaseNode,
  /// Window duration (wall-clock time between emissions).
  window_size: Duration,
}

impl TumblingProcessingTimeWindowNode {
  /// Creates a new TumblingProcessingTimeWindowNode with the given name and window size.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of the node.
  /// * `window_size` - Duration of each window (e.g. `Duration::from_secs(1)`).
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::stream::TumblingProcessingTimeWindowNode;
  /// use std::time::Duration;
  ///
  /// let node = TumblingProcessingTimeWindowNode::new(
  ///     "tumbling_window".to_string(),
  ///     Duration::from_millis(1000),
  /// );
  /// // Creates ports: configuration, in -> out, error
  /// ```
  pub fn new(name: String, window_size: Duration) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec!["configuration".to_string(), "in".to_string()],
        vec!["out".to_string(), "error".to_string()],
      ),
      window_size,
    }
  }

  /// Returns the window size.
  pub fn window_size(&self) -> Duration {
    self.window_size
  }
}

#[async_trait]
impl Node for TumblingProcessingTimeWindowNode {
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
    let window_size = self.window_size;
    Box::pin(async move {
      let _config_stream = inputs.remove("configuration");
      let in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;

      let (out_tx, out_rx) = mpsc::channel(10);
      let (_error_tx, error_rx) = mpsc::channel(10);

      tokio::spawn(async move {
        let mut in_stream = in_stream;
        let mut current_window: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
        let mut ticker = interval(window_size);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
          tokio::select! {
              item = in_stream.next() => {
                  match item {
                      Some(arc) => current_window.push(arc),
                      None => {
                          if !current_window.is_empty() {
                              let window = std::mem::take(&mut current_window);
                              let _ = out_tx
                                  .send(Arc::new(window) as Arc<dyn Any + Send + Sync>)
                                  .await;
                          }
                          break;
                      }
                  }
              }
              _ = ticker.tick() => {
                  if !current_window.is_empty() {
                      let window = std::mem::take(&mut current_window);
                      let _ = out_tx
                          .send(Arc::new(window) as Arc<dyn Any + Send + Sync>)
                          .await;
                  }
              }
          }
        }
      });

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
