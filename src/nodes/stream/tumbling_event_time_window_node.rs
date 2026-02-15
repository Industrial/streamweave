//! # Tumbling Event-Time Window Node
//!
//! A transform node that collects items into fixed-duration windows based on
//! event time (logical time) and closes them on watermark.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration (currently unused, for consistency)
//! - **Input**: `"in"` - Receives `StreamMessage<Timestamped<Arc<dyn Any>>>` (Data or Watermark)
//! - **Output**: `"out"` - Sends closed windows as `Vec<Arc<dyn Any + Send + Sync>>`
//! - **Output**: `"error"` - Sends errors that occur during processing
//!
//! ## Input convention
//!
//! The node expects items that are either:
//! - `Arc<StreamMessage<Timestamped<Arc<dyn Any>>>>` – Data or Watermark
//! - `Arc<Timestamped<Arc<dyn Any>>>` – treated as Data with that time
//! - `Arc<dyn Any>` (other) – treated as Data with `LogicalTime::minimum()`
//!
//! ## Behavior
//!
//! - Assignment: item with event time T → window `[floor(T/size)*size, floor(T/size)*size + size)`
//! - On Watermark(T): close all windows with `window_end <= T`, emit buffers
//! - Late data (event time < watermark) is dropped
//!
//! See [docs/windowing.md](../../../docs/windowing.md) §5.1.

#![allow(clippy::type_complexity)]

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use crate::nodes::stream::LateDataPolicy;
use crate::time::{LogicalTime, StreamMessage, Timestamped};
use async_trait::async_trait;
use std::any::Any;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

/// A node that emits tumbling windows based on event time, closing on watermark.
///
/// Buffers items by window; on Watermark(T), closes windows with end <= T and emits.
/// Requires upstream to emit `StreamMessage` (Data or Watermark). Works with
/// `Timestamped`-only streams (no watermarks = windows never close until EOS).
///
/// Late data (event_time before watermark) is handled per [`LateDataPolicy`].
pub struct TumblingEventTimeWindowNode {
    /// Base node functionality.
    pub(crate) base: BaseNode,
    /// Window duration (event-time units: LogicalTime.as_u64() as milliseconds).
    window_size: Duration,
    /// Policy for late data (events with event_time < watermark).
    late_data_policy: LateDataPolicy,
}

impl TumblingEventTimeWindowNode {
    /// Creates a new TumblingEventTimeWindowNode with the given name and window size.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the node.
    /// * `window_size` - Duration of each window (e.g. `Duration::from_secs(3600)`).
    ///   LogicalTime values are interpreted as milliseconds for alignment.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use streamweave::nodes::stream::TumblingEventTimeWindowNode;
    /// use std::time::Duration;
    ///
    /// let node = TumblingEventTimeWindowNode::new(
    ///     "event_time_window".to_string(),
    ///     Duration::from_secs(3600),
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

    /// Returns the window size.
    pub fn window_size(&self) -> Duration {
        self.window_size
    }
}

/// Attempts to extract StreamMessage from an Arc<dyn Any>.
fn try_stream_message(
    item: Arc<dyn Any + Send + Sync>,
) -> Option<StreamMessage<Arc<dyn Any + Send + Sync>>> {
    item.downcast::<StreamMessage<Arc<dyn Any + Send + Sync>>>()
        .ok()
        .map(|arc| (*arc).clone())
}

/// Attempts to extract Timestamped from an Arc<dyn Any>.
fn try_timestamped(
    item: Arc<dyn Any + Send + Sync>,
) -> Option<Timestamped<Arc<dyn Any + Send + Sync>>> {
    item.downcast::<Timestamped<Arc<dyn Any + Send + Sync>>>()
        .ok()
        .map(|arc| (*arc).clone())
}

/// Window start for event time T and window size in milliseconds.
fn window_start_ms(t_ms: u64, size_ms: u64) -> u64 {
    if size_ms == 0 {
        return t_ms;
    }
    (t_ms / size_ms) * size_ms
}

#[async_trait]
impl Node for TumblingEventTimeWindowNode {
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
        let late_data_policy = self.late_data_policy;
        Box::pin(async move {
            let _config_stream = inputs.remove("configuration");
            let in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;

            let (out_tx, out_rx) = mpsc::channel(10);
            let (_error_tx, error_rx) = mpsc::channel(10);
            let (late_tx, late_rx) = if late_data_policy.use_side_output() {
                let (tx, rx) = mpsc::channel(10);
                (Some(tx), Some(rx))
            } else {
                (None, None)
            };

            tokio::spawn(async move {
                let size_ms = window_size.as_millis() as u64;
                let mut in_stream = in_stream;
                let mut windows: BTreeMap<u64, Vec<Arc<dyn Any + Send + Sync>>> = BTreeMap::new();
                let mut last_watermark = LogicalTime::minimum();

                loop {
                    match in_stream.next().await {
                        None => {
                            // EOS: flush all remaining windows
                            let remaining = std::mem::take(&mut windows);
                            for (_, buf) in remaining {
                                if !buf.is_empty() {
                                    let _ = out_tx
                                        .send(Arc::new(buf) as Arc<dyn Any + Send + Sync>)
                                        .await;
                                }
                            }
                            break;
                        }
                        Some(item) => {
                            if let Some(msg) = try_stream_message(item.clone()) {
                                match msg {
                                    StreamMessage::Data(ts) => {
                                        let t = ts.time().as_u64();
                                        if size_ms == 0 {
                                            let _ = out_tx
                                                .send(Arc::new(vec![ts.payload]) as Arc<dyn Any + Send + Sync>)
                                                .await;
                                        } else {
                                            let start = window_start_ms(t, size_ms);
                                            let window_end = start + size_ms;
                                            if last_watermark.as_u64() < window_end {
                                                windows.entry(start).or_default().push(ts.payload);
                                            } else if late_data_policy.use_side_output() {
                                                let _ = late_tx.as_ref().unwrap().send(ts.payload).await;
                                            }
                                        }
                                    }
                                    StreamMessage::Watermark(w) => {
                                        last_watermark = w;
                                        let w_ms = w.as_u64();
                                        let to_close: Vec<u64> = windows
                                            .keys()
                                            .copied()
                                            .filter(|&start| start + size_ms <= w_ms)
                                            .collect();
                                        for start in to_close {
                                            if let Some(buf) = windows.remove(&start) {
                                                if !buf.is_empty() {
                                                    let _ = out_tx
                                                        .send(Arc::new(buf) as Arc<dyn Any + Send + Sync>)
                                                        .await;
                                                }
                                            }
                                        }
                                    }
                                }
                            } else if let Some(ts) = try_timestamped(item.clone()) {
                                let t = ts.time().as_u64();
                                if size_ms == 0 {
                                    let _ = out_tx
                                        .send(Arc::new(vec![ts.payload]) as Arc<dyn Any + Send + Sync>)
                                        .await;
                                } else {
                                    let start = window_start_ms(t, size_ms);
                                    let window_end = start + size_ms;
                                    if last_watermark.as_u64() < window_end {
                                        windows.entry(start).or_default().push(ts.payload);
                                    } else if late_data_policy.use_side_output() {
                                        let _ = late_tx.as_ref().unwrap().send(ts.payload).await;
                                    }
                                }
                            } else {
                                // Legacy: treat as Data with time 0
                                if size_ms == 0 {
                                    let _ = out_tx
                                        .send(Arc::new(vec![item]) as Arc<dyn Any + Send + Sync>)
                                        .await;
                                } else {
                                    windows
                                        .entry(0)
                                        .or_default()
                                        .push(item);
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
