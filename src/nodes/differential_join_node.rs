//! # Differential Join Node
//!
//! Equi-join over two differential streams. Outputs (key, (left, right), time, diff)
//! as the **changes** to the join result. Output diff = left_diff * right_diff.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` (Arc<JoinConfig>), `"left"`, `"right"` (DifferentialStreamMessage)
//! - **Output**: `"out"`, `"error"`
//!
//! See [docs/timestamped-differential-dataflow.md](../docs/timestamped-differential-dataflow.md) §4.3.

#![allow(clippy::type_complexity)]

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use crate::nodes::join_node::JoinConfig;
use crate::time::{DifferentialElement, DifferentialStreamMessage, LogicalTime};
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

#[derive(Clone, Copy)]
enum Port {
    Config,
    Left,
    Right,
}

fn try_differential_message(
    item: &Arc<dyn Any + Send + Sync>,
) -> Option<DifferentialStreamMessage<Arc<dyn Any + Send + Sync>>> {
    item.clone()
        .downcast::<DifferentialStreamMessage<Arc<dyn Any + Send + Sync>>>()
        .ok()
        .map(|arc| (*arc).clone())
}

/// Differential equi-join: outputs changes only. Output diff = left_diff * right_diff.
pub struct DifferentialJoinNode {
    pub(crate) base: BaseNode,
    current_config: Arc<Mutex<Option<Arc<JoinConfig>>>>,
}

impl DifferentialJoinNode {
    /// Creates a new DifferentialJoinNode.
    pub fn new(name: String) -> Self {
        Self {
            base: BaseNode::new(
                name,
                vec![
                    "configuration".to_string(),
                    "left".to_string(),
                    "right".to_string(),
                ],
                vec!["out".to_string(), "error".to_string()],
            ),
            current_config: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait]
impl Node for DifferentialJoinNode {
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

        Box::pin(async move {
            let config_stream = inputs.remove("configuration").ok_or("Missing 'configuration' input")?;
            let left_stream = inputs.remove("left").ok_or("Missing 'left' input")?;
            let right_stream = inputs.remove("right").ok_or("Missing 'right' input")?;

            let config_tagged = config_stream.map(|i| (Port::Config, i));
            let left_tagged = left_stream.map(|i| (Port::Left, i));
            let right_tagged = right_stream.map(|i| (Port::Right, i));
            type MergedItem = (Port, Arc<dyn Any + Send + Sync>);
            let merged: Pin<Box<dyn futures::Stream<Item = MergedItem> + Send>> = Box::pin(
                stream::select_all(vec![
                    Box::pin(config_tagged) as Pin<Box<dyn futures::Stream<Item = MergedItem> + Send>>,
                    Box::pin(left_tagged) as Pin<Box<dyn futures::Stream<Item = MergedItem> + Send>>,
                    Box::pin(right_tagged) as Pin<Box<dyn futures::Stream<Item = MergedItem> + Send>>,
                ]),
            );

            let (out_tx, out_rx) = mpsc::channel(10);
            let (error_tx, error_rx) = mpsc::channel(10);

            tokio::spawn(async move {
                let mut merged = merged;
                let mut left_buf: HashMap<String, Vec<(Arc<dyn Any + Send + Sync>, LogicalTime, i64)>> =
                    HashMap::new();
                let mut right_buf: HashMap<String, Vec<(Arc<dyn Any + Send + Sync>, LogicalTime, i64)>> =
                    HashMap::new();
                let mut config: Option<Arc<JoinConfig>> = None;

                while let Some((port, item)) = merged.next().await {
                    match port {
                        Port::Config => {
                            if let Ok(arc) = item.clone().downcast::<Arc<JoinConfig>>() {
                                config = Some(Arc::clone(&*arc));
                                *config_state.lock().await = Some(Arc::clone(&*arc));
                            } else {
                                let _ = error_tx
                                    .send(Arc::new("Invalid configuration type - expected Arc<JoinConfig>".to_string())
                                        as Arc<dyn Any + Send + Sync>)
                                    .await;
                            }
                        }
                        Port::Left | Port::Right => {
                            let Some(cfg) = &config else {
                                let _ = error_tx
                                    .send(Arc::new("No configuration set. Please send configuration before data.".to_string())
                                        as Arc<dyn Any + Send + Sync>)
                                    .await;
                                continue;
                            };
                            let Some(msg) = try_differential_message(&item) else {
                                continue;
                            };
                            match msg {
                                DifferentialStreamMessage::Data(elem) => {
                                    let (key_res, is_left) = if matches!(port, Port::Left) {
                                        (
                                            cfg.left_key_fn.extract_key(elem.payload().clone()).await,
                                            true,
                                        )
                                    } else {
                                        (
                                            cfg.right_key_fn.extract_key(elem.payload().clone()).await,
                                            false,
                                        )
                                    };
                                    let key = match key_res {
                                        Ok(k) => k,
                                        Err(e) => {
                                            let _ = error_tx
                                                .send(Arc::new(e) as Arc<dyn Any + Send + Sync>)
                                                .await;
                                            continue;
                                        }
                                    };
                                    let time = elem.time();
                                    let diff = elem.diff();
                                    let payload = elem.payload().clone();

                                    if is_left {
                                        left_buf
                                            .entry(key.clone())
                                            .or_default()
                                            .push((payload.clone(), time, diff));
                                        for (rv, _rt, rd) in right_buf.get(&key).cloned().unwrap_or_default() {
                                            match cfg
                                                .combine_fn
                                                .combine(payload.clone(), Some(rv.clone()))
                                                .await
                                            {
                                                Ok(joined) => {
                                                    let out_diff = diff * rd;
                                                    if out_diff != 0 {
                                                        let out_elem =
                                                            DifferentialElement::new(joined, time, out_diff);
                                                        let _ = out_tx
                                                            .send(Arc::new(DifferentialStreamMessage::Data(out_elem))
                                                                as Arc<dyn Any + Send + Sync>)
                                                            .await;
                                                    }
                                                }
                                                Err(e) => {
                                                    let _ = error_tx
                                                        .send(Arc::new(e) as Arc<dyn Any + Send + Sync>)
                                                        .await;
                                                }
                                            }
                                        }
                                    } else {
                                        right_buf
                                            .entry(key.clone())
                                            .or_default()
                                            .push((payload.clone(), time, diff));
                                        for (lv, _lt, ld) in left_buf.get(&key).cloned().unwrap_or_default() {
                                            match cfg
                                                .combine_fn
                                                .combine(lv.clone(), Some(payload.clone()))
                                                .await
                                            {
                                                Ok(joined) => {
                                                    let out_diff = ld * diff;
                                                    if out_diff != 0 {
                                                        let out_elem =
                                                            DifferentialElement::new(joined, time, out_diff);
                                                        let _ = out_tx
                                                            .send(Arc::new(DifferentialStreamMessage::Data(out_elem))
                                                                as Arc<dyn Any + Send + Sync>)
                                                            .await;
                                                    }
                                                }
                                                Err(e) => {
                                                    let _ = error_tx
                                                        .send(Arc::new(e) as Arc<dyn Any + Send + Sync>)
                                                        .await;
                                                }
                                            }
                                        }
                                    }
                                }
                                DifferentialStreamMessage::Watermark(w) => {
                                    let wm = DifferentialStreamMessage::<Arc<dyn Any + Send + Sync>>::Watermark(w);
                                    let _ = out_tx.send(Arc::new(wm) as Arc<dyn Any + Send + Sync>).await;
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
            Ok(outputs)
        })
    }
}
