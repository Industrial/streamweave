//! # Watermark Injector Node
//!
//! A transform node that converts a stream of data into `StreamMessage` (Data + Watermark)
//! for consumption by event-time window nodes.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives data items (Timestamped, StreamMessage, or plain with event_timestamp)
//! - **Output**: `"out"` - Sends `StreamMessage` (Data wrapped as Timestamped, Watermark on EOS)
//! - **Output**: `"error"` - Sends errors
//!
//! ## Behavior
//!
//! - For each data item: extracts event time, wraps as `StreamMessage::Data(Timestamped{payload, time})`,
//!   updates internal max_time.
//! - On end-of-stream: emits `StreamMessage::Watermark(max_time)` so downstream windows can close.
//!
//! ## Input conventions
//!
//! - `Arc<StreamMessage<...>>` – pass through Data, track time; forward Watermarks.
//! - `Arc<Timestamped<...>>` – treat as Data, use its time.
//! - `Arc<dyn Any>` (HashMap with `event_timestamp` or `timestamp`) – use that as event time.
//! - Otherwise – use `LogicalTime::minimum()`.
//!
//! See [docs/progress-tracking.md](../../../docs/progress-tracking.md) phase 4.

#![allow(clippy::type_complexity)]

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use crate::time::{LogicalTime, StreamMessage, Timestamped};
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Attempts to extract StreamMessage from an Arc<dyn Any>.
fn try_stream_message(
  item: Arc<dyn Any + Send + Sync>,
) -> Option<StreamMessage<Arc<dyn Any + Send + Sync>>> {
  item
    .downcast::<StreamMessage<Arc<dyn Any + Send + Sync>>>()
    .ok()
    .map(|arc| (*arc).clone())
}

/// Attempts to extract Timestamped from an Arc<dyn Any>.
fn try_timestamped(
  item: Arc<dyn Any + Send + Sync>,
) -> Option<Timestamped<Arc<dyn Any + Send + Sync>>> {
  item
    .downcast::<Timestamped<Arc<dyn Any + Send + Sync>>>()
    .ok()
    .map(|arc| (*arc).clone())
}

/// Extracts event time (u64) from a HashMap with "event_timestamp" or "timestamp".
fn event_time_from_payload(item: &Arc<dyn Any + Send + Sync>) -> Option<u64> {
  let arc_map = item
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
    .ok()?;
  let map = arc_map.as_ref();
  let ts = map
    .get("event_timestamp")
    .or_else(|| map.get("timestamp"))?;
  if let Ok(arc_i64) = ts.clone().downcast::<i64>() {
    return Some(*arc_i64 as u64);
  }
  if let Ok(arc_u64) = ts.clone().downcast::<u64>() {
    return Some(*arc_u64);
  }
  None
}

/// A node that injects watermarks into a stream for event-time window consumption.
///
/// Accepts Timestamped, StreamMessage, or payloads with event_timestamp; outputs
/// StreamMessage with Data and Watermark(max_time) on EOS.
pub struct WatermarkInjectorNode {
  /// Shared base node (ports, name).
  pub(crate) base: BaseNode,
}

impl WatermarkInjectorNode {
  /// Creates a new WatermarkInjectorNode.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::stream::WatermarkInjectorNode;
  ///
  /// let node = WatermarkInjectorNode::new("watermark".to_string());
  /// // Ports: configuration, in -> out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec!["configuration".to_string(), "in".to_string()],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for WatermarkInjectorNode {
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
      let _config_stream = inputs.remove("configuration");
      let in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;

      let (out_tx, out_rx) = mpsc::channel(10);
      let (_error_tx, error_rx) = mpsc::channel(10);

      tokio::spawn(async move {
        let mut in_stream = in_stream;
        let mut max_time: u64 = 0;

        loop {
          match in_stream.next().await {
            None => {
              if max_time > 0 {
                let _ = out_tx
                  .send(
                    Arc::new(StreamMessage::<Arc<dyn Any + Send + Sync>>::Watermark(
                      LogicalTime::new(max_time),
                    )) as Arc<dyn Any + Send + Sync>,
                  )
                  .await;
              }
              break;
            }
            Some(item) => {
              if let Some(msg) = try_stream_message(item.clone()) {
                match msg {
                  StreamMessage::Data(ts) => {
                    let t = ts.time().as_u64();
                    max_time = max_time.max(t);
                    let out = Arc::new(StreamMessage::Data(ts)) as Arc<dyn Any + Send + Sync>;
                    let _ = out_tx.send(out).await;
                  }
                  StreamMessage::Watermark(w) => {
                    let t = w.as_u64();
                    max_time = max_time.max(t);
                    let out = Arc::new(StreamMessage::<Arc<dyn Any + Send + Sync>>::Watermark(w))
                      as Arc<dyn Any + Send + Sync>;
                    let _ = out_tx.send(out).await;
                  }
                }
              } else if let Some(ts) = try_timestamped(item.clone()) {
                let t = ts.time().as_u64();
                max_time = max_time.max(t);
                let out = Arc::new(StreamMessage::Data(ts)) as Arc<dyn Any + Send + Sync>;
                let _ = out_tx.send(out).await;
              } else {
                let t = event_time_from_payload(&item).unwrap_or(0);
                max_time = max_time.max(t);
                let ts = Timestamped::new(item, LogicalTime::new(t));
                let out = Arc::new(StreamMessage::Data(ts)) as Arc<dyn Any + Send + Sync>;
                let _ = out_tx.send(out).await;
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
