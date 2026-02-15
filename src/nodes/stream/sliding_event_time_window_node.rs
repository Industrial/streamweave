//! # Sliding Event-Time Window Node
//!
//! A transform node that collects items into overlapping fixed-size windows based on
//! event time, closing on watermark.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"`, `"in"` – same as [`TumblingEventTimeWindowNode`]
//! - **Output**: `"out"`, `"error"`
//!
//! ## Behavior
//!
//! - Windows: [0, size), [slide, size+slide), [2*slide, size+2*slide), ...
//! - Assignment: item with event time T goes to all windows [start, start+size) containing T
//! - On Watermark(T): close windows with end <= T, emit buffers
//!
//! See [docs/windowing.md](../../../docs/windowing.md) §5.2.

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

/// Returns window starts (ms) that contain T: [start, start+size) with start = k*slide.
fn window_starts_for_t(t_ms: u64, size_ms: u64, slide_ms: u64) -> Vec<u64> {
    if size_ms == 0 || slide_ms == 0 {
        return vec![t_ms];
    }
    // start > T - size  =>  k > (T - size) / slide
    // start <= T        =>  k <= T / slide
    let k_min = if t_ms >= size_ms {
        (t_ms - size_ms) / slide_ms + 1
    } else {
        0
    };
    let k_max = t_ms / slide_ms;
    (k_min..=k_max).map(|k| k * slide_ms).collect()
}

/// Sliding event-time window node: overlapping windows, close on watermark.
///
/// Late data (event_time before watermark) is handled per [`LateDataPolicy`].
pub struct SlidingEventTimeWindowNode {
    pub(crate) base: BaseNode,
    window_size: Duration,
    slide: Duration,
    late_data_policy: LateDataPolicy,
}

impl SlidingEventTimeWindowNode {
    /// Creates a new SlidingEventTimeWindowNode.
    ///
    /// # Arguments
    /// * `name` - Node name
    /// * `window_size` - Window duration
    /// * `slide` - Step between window starts (must be <= window_size for overlap)
    pub fn new(name: String, window_size: Duration, slide: Duration) -> Self {
        Self {
            base: BaseNode::new(
                name,
                vec!["configuration".to_string(), "in".to_string()],
                vec!["out".to_string(), "error".to_string()],
            ),
            window_size,
            slide,
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

    /// Returns the window size duration.
    pub fn window_size(&self) -> Duration {
        self.window_size
    }

    /// Returns the slide interval duration.
    pub fn slide(&self) -> Duration {
        self.slide
    }
}

fn try_stream_message(
    item: Arc<dyn Any + Send + Sync>,
) -> Option<StreamMessage<Arc<dyn Any + Send + Sync>>> {
    item.downcast::<StreamMessage<Arc<dyn Any + Send + Sync>>>()
        .ok()
        .map(|arc| (*arc).clone())
}

fn try_timestamped(
    item: Arc<dyn Any + Send + Sync>,
) -> Option<Timestamped<Arc<dyn Any + Send + Sync>>> {
    item.downcast::<Timestamped<Arc<dyn Any + Send + Sync>>>()
        .ok()
        .map(|arc| (*arc).clone())
}

#[async_trait]
impl Node for SlidingEventTimeWindowNode {
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
        let size_ms = self.window_size.as_millis() as u64;
        let slide_ms = self.slide.as_millis() as u64;
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
                let mut windows: BTreeMap<u64, Vec<Arc<dyn Any + Send + Sync>>> = BTreeMap::new();
                let mut last_watermark = LogicalTime::minimum();
                let mut in_stream = in_stream;

                loop {
                    match in_stream.next().await {
                        None => {
                            let remaining = std::mem::take(&mut windows);
                            for (_, buf) in remaining {
                                if !buf.is_empty() {
                                    let _ = out_tx.send(Arc::new(buf) as Arc<dyn Any + Send + Sync>).await;
                                }
                            }
                            break;
                        }
                        Some(item) => {
                            let (payload_opt, event_time, is_watermark) = if let Some(msg) =
                                try_stream_message(item.clone())
                            {
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
                                let to_close: Vec<u64> = windows
                                    .keys()
                                    .copied()
                                    .filter(|&start| start + size_ms <= event_time)
                                    .collect();
                                for start in to_close {
                                    if let Some(buf) = windows.remove(&start) {
                                        if !buf.is_empty() {
                                            let _ =
                                                out_tx.send(Arc::new(buf) as Arc<dyn Any + Send + Sync>).await;
                                        }
                                    }
                                }
                            } else if let Some(payload) = payload_opt {
                                if size_ms == 0 || slide_ms == 0 {
                                    let _ = out_tx
                                        .send(Arc::new(vec![payload]) as Arc<dyn Any + Send + Sync>)
                                        .await;
                                } else {
                                    let starts = window_starts_for_t(event_time, size_ms, slide_ms);
                                    let mut any_added = false;
                                    for start in starts {
                                        let window_end = start + size_ms;
                                        if last_watermark.as_u64() < window_end {
                                            windows.entry(start).or_default().push(payload.clone());
                                            any_added = true;
                                        }
                                    }
                                    if !any_added && late_data_policy.use_side_output() {
                                        let _ = late_tx.as_ref().unwrap().send(payload).await;
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
