//! # Session Event-Time Window Node
//!
//! A transform node that groups items into sessions based on a gap in event time.
//! A new session starts when an event arrives more than `gap` after the previous event.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"`, `"in"` – same as [`TumblingEventTimeWindowNode`]
//! - **Output**: `"out"`, `"error"`
//!
//! ## Behavior
//!
//! - Non-keyed: single logical stream; sessions are (start, end, items)
//! - New session when event_time - last_event_time > gap
//! - On Watermark(T): close session if last_event_time + gap < T
//!
//! See [docs/windowing.md](../../../docs/windowing.md) §5.3.

#![allow(clippy::type_complexity)]

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use crate::nodes::stream::LateDataPolicy;
use crate::time::{LogicalTime, StreamMessage, Timestamped};
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Session event-time window node: gap-based sessions, close on watermark.
///
/// Late data (event_time before watermark) is handled per [`LateDataPolicy`].
pub struct SessionEventTimeWindowNode {
  pub(crate) base: BaseNode,
  gap: Duration,
  late_data_policy: LateDataPolicy,
}

impl SessionEventTimeWindowNode {
  /// Creates a new SessionEventTimeWindowNode.
  ///
  /// # Arguments
  /// * `name` - Node name
  /// * `gap` - Maximum gap between events in the same session
  pub fn new(name: String, gap: Duration) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec!["configuration".to_string(), "in".to_string()],
        vec!["out".to_string(), "error".to_string()],
      ),
      gap,
      late_data_policy: LateDataPolicy::default(),
    }
  }

  /// Configures how late data (event_time &lt; watermark) is handled.
  pub fn with_late_data_policy(mut self, policy: LateDataPolicy) -> Self {
    self.late_data_policy = policy;
    if policy.use_side_output() {
      self.base.output_port_names.push("late".to_string());
    }
    self
  }

  /// Returns the session gap duration.
  pub fn gap(&self) -> Duration {
    self.gap
  }
}

fn try_stream_message(
  item: Arc<dyn Any + Send + Sync>,
) -> Option<StreamMessage<Arc<dyn Any + Send + Sync>>> {
  item
    .downcast::<StreamMessage<Arc<dyn Any + Send + Sync>>>()
    .ok()
    .map(|arc| (*arc).clone())
}

fn try_timestamped(
  item: Arc<dyn Any + Send + Sync>,
) -> Option<Timestamped<Arc<dyn Any + Send + Sync>>> {
  item
    .downcast::<Timestamped<Arc<dyn Any + Send + Sync>>>()
    .ok()
    .map(|arc| (*arc).clone())
}

#[async_trait]
impl Node for SessionEventTimeWindowNode {
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
    let gap_ms = self.gap.as_millis() as u64;
    let late_data_policy = self.late_data_policy;

    Box::pin(async move {
      let _config = inputs.remove("configuration");
      let in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;

      let (out_tx, out_rx) = mpsc::channel(10);
      let (_err_tx, error_rx) = mpsc::channel(10);
      let (late_tx, late_rx) = if late_data_policy.use_side_output() {
        let (tx, rx) = mpsc::channel(10);
        (Some(tx), Some(rx))
      } else {
        (None, None)
      };

      tokio::spawn(async move {
        let mut session: Option<(u64, u64, Vec<Arc<dyn Any + Send + Sync>>)> = None;
        let mut last_watermark = LogicalTime::minimum();
        let mut in_stream = in_stream;

        loop {
          match in_stream.next().await {
            None => {
              if let Some((_, _, buf)) = session.take()
                && !buf.is_empty()
              {
                let _ = out_tx
                  .send(Arc::new(buf) as Arc<dyn Any + Send + Sync>)
                  .await;
              }
              break;
            }
            Some(item) => {
              let (payload_opt, event_time, is_watermark) =
                if let Some(msg) = try_stream_message(item.clone()) {
                  match msg {
                    StreamMessage::Data(ts) => {
                      let t = ts.time().as_u64();
                      (Some(ts.payload), t, false)
                    }
                    StreamMessage::Watermark(w) => (None, w.as_u64(), true),
                  }
                } else if let Some(ts) = try_timestamped(item.clone()) {
                  let t = ts.time().as_u64();
                  (Some(ts.payload), t, false)
                } else {
                  (Some(item), 0, false)
                };

              if is_watermark {
                last_watermark = LogicalTime::new(event_time);
                if let Some((start, last, buf)) = session.take() {
                  if last + gap_ms < event_time {
                    if !buf.is_empty() {
                      let _ = out_tx
                        .send(Arc::new(buf) as Arc<dyn Any + Send + Sync>)
                        .await;
                    }
                  } else {
                    session = Some((start, last, buf));
                  }
                }
              } else if let Some(payload) = payload_opt {
                if event_time < last_watermark.as_u64() {
                  if late_data_policy.use_side_output() {
                    let _ = late_tx.as_ref().unwrap().send(payload).await;
                  }
                } else {
                  let start_new = session
                    .as_ref()
                    .is_none_or(|(_, last, _)| event_time.saturating_sub(*last) > gap_ms);
                  if start_new {
                    if let Some((_, _, buf)) = session.take()
                      && !buf.is_empty()
                    {
                      let _ = out_tx
                        .send(Arc::new(buf) as Arc<dyn Any + Send + Sync>)
                        .await;
                    }
                    session = Some((event_time, event_time, vec![payload]));
                  } else {
                    let (start, _, mut buf) = session.take().unwrap();
                    buf.push(payload);
                    session = Some((start, event_time, buf));
                  }
                }
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
      if let Some(rx) = late_rx {
        outputs.insert(
          "late".to_string(),
          Box::pin(ReceiverStream::new(rx))
            as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
        );
      }
      Ok(outputs)
    })
  }
}
