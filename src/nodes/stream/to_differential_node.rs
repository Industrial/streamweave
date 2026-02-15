//! # To-Differential Node
//!
//! Wraps items from a non-differential stream as [`DifferentialElement`] with diff = +1.
//! Enables feeding regular nodes into differential operators.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"`, `"in"`
//! - **Output**: `"out"`, `"error"`
//!
//! ## Input convention
//!
//! - `Arc<StreamMessage<Timestamped<Arc<dyn Any>>>>` – Data uses ts.time; Watermark forwarded
//! - `Arc<Timestamped<Arc<dyn Any>>>` – uses ts.time, diff +1
//! - `Arc<dyn Any>` (other) – uses monotonic logical time, diff +1
//!
//! See [docs/timestamped-differential-dataflow.md](../../../docs/timestamped-differential-dataflow.md) §4.2.

#![allow(clippy::type_complexity)]

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use crate::time::{DifferentialElement, DifferentialStreamMessage, LogicalTime, StreamMessage, Timestamped};
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

fn try_stream_message(item: &Arc<dyn Any + Send + Sync>) -> Option<StreamMessage<Arc<dyn Any + Send + Sync>>> {
    item.clone()
        .downcast::<StreamMessage<Arc<dyn Any + Send + Sync>>>()
        .ok()
        .map(|arc| (*arc).clone())
}

fn try_timestamped(item: &Arc<dyn Any + Send + Sync>) -> Option<Timestamped<Arc<dyn Any + Send + Sync>>> {
    item.clone()
        .downcast::<Timestamped<Arc<dyn Any + Send + Sync>>>()
        .ok()
        .map(|arc| (*arc).clone())
}

/// Wraps each input item as `DifferentialElement::insert(payload, time, +1)`.
pub struct ToDifferentialNode {
    pub(crate) base: BaseNode,
}

impl ToDifferentialNode {
    /// Creates a new ToDifferentialNode.
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
impl Node for ToDifferentialNode {
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
            let _config = inputs.remove("configuration");
            let in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;

            let (out_tx, out_rx) = mpsc::channel(10);
            let (_err_tx, error_rx) = mpsc::channel(10);
            let counter = Arc::new(AtomicU64::new(0));

            tokio::spawn(async move {
                let mut in_stream = in_stream;
                while let Some(item) = in_stream.next().await {
                    let (payload_opt, time, is_watermark) = if let Some(msg) = try_stream_message(&item) {
                        match msg {
                            StreamMessage::Data(ts) => {
                                let t = ts.time();
                                (Some(ts.payload), t, false)
                            }
                            StreamMessage::Watermark(w) => (None, w, true),
                        }
                    } else if let Some(ts) = try_timestamped(&item) {
                        let t = ts.time();
                        (Some(ts.payload), t, false)
                    } else {
                        let t = LogicalTime::new(counter.fetch_add(1, Ordering::SeqCst));
                        (Some(item), t, false)
                    };

                    if is_watermark {
                        let wm = Arc::new(DifferentialStreamMessage::<Arc<dyn Any + Send + Sync>>::Watermark(time))
                            as Arc<dyn Any + Send + Sync>;
                        let _ = out_tx.send(wm).await;
                    } else if let Some(payload) = payload_opt {
                        let elem = DifferentialElement::insert(payload, time);
                        let msg = Arc::new(DifferentialStreamMessage::Data(elem)) as Arc<dyn Any + Send + Sync>;
                        let _ = out_tx.send(msg).await;
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
