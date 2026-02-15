//! # Differential Group-By Node
//!
//! Incremental count aggregation over a differential stream. Consumes
//! `DifferentialStreamMessage` and emits (key, count, time, diff) where output
//! diff is the change in the count for that key at that time.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"`, `"in"` (DifferentialStreamMessage), `"key_function"`
//! - **Output**: `"out"`, `"error"`
//!
//! See [docs/timestamped-differential-dataflow.md](../../../docs/timestamped-differential-dataflow.md) §4.3.

#![allow(clippy::type_complexity)]

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use crate::nodes::reduction::{GroupByConfig, GroupByConfigWrapper};
use crate::time::{DifferentialElement, DifferentialStreamMessage};
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Input port names for the group-by node.
#[allow(dead_code)]
enum InputPort {
  /// Configuration port.
  Config,
  /// Main data input.
  In,
  /// Key extraction function port.
  KeyFunction,
}

/// Downcasts an item to `DifferentialStreamMessage` if possible.
fn try_differential_message(
  item: &Arc<dyn Any + Send + Sync>,
) -> Option<DifferentialStreamMessage<Arc<dyn Any + Send + Sync>>> {
  item
    .clone()
    .downcast::<DifferentialStreamMessage<Arc<dyn Any + Send + Sync>>>()
    .ok()
    .map(|arc| (*arc).clone())
}

/// Differential group-by with count aggregation. Outputs (key, count, time, diff).
pub struct DifferentialGroupByNode {
  /// Shared base node (ports, name).
  pub(crate) base: BaseNode,
}

impl DifferentialGroupByNode {
  /// Creates a new DifferentialGroupByNode.
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "in".to_string(),
          "key_function".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
    }
  }
}

#[async_trait]
impl Node for DifferentialGroupByNode {
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
      let key_fn_stream = inputs
        .remove("key_function")
        .ok_or("Missing 'key_function' input")?;

      let in_stream = in_stream.map(|item| (InputPort::In, item));
      let key_fn_stream = key_fn_stream.map(|item| (InputPort::KeyFunction, item));
      let merged: Pin<
        Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>,
      > = Box::pin(stream::select_all(vec![
        Box::pin(in_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
        Box::pin(key_fn_stream)
          as Pin<Box<dyn futures::Stream<Item = (InputPort, Arc<dyn Any + Send + Sync>)> + Send>>,
      ]));

      let (out_tx, out_rx) = mpsc::channel(10);
      let (error_tx, error_rx) = mpsc::channel(10);

      tokio::spawn(async move {
        let mut merged = merged;
        let mut state: HashMap<String, i64> = HashMap::new();
        let mut key_fn: Option<GroupByConfig> = None;
        let mut buffer: Vec<(InputPort, Arc<dyn Any + Send + Sync>)> = Vec::new();

        while let Some((port, item)) = merged.next().await {
          match port {
            InputPort::Config => {}
            InputPort::KeyFunction => {
              if let Ok(wrapper) = Arc::downcast::<GroupByConfigWrapper>(item.clone()) {
                key_fn = Some(wrapper.0.clone());
                for (_, buffered) in buffer.drain(..) {
                  if let Some(msg) = try_differential_message(&buffered) {
                    match msg {
                      DifferentialStreamMessage::Data(elem) => {
                        if let Some(kf) = &key_fn
                          && let Ok(key) = kf.extract_key(elem.payload().clone()).await
                        {
                          let time = elem.time();
                          let diff = elem.diff();
                          let count = state.entry(key.clone()).or_insert(0);
                          *count += diff;
                          let out_elem = DifferentialElement::new(
                            Arc::new((key, *count)) as Arc<dyn Any + Send + Sync>,
                            time,
                            diff,
                          );
                          let _ = out_tx
                            .send(Arc::new(DifferentialStreamMessage::Data(out_elem))
                              as Arc<dyn Any + Send + Sync>)
                            .await;
                        }
                      }
                      DifferentialStreamMessage::Watermark(w) => {
                        let wm =
                          DifferentialStreamMessage::<Arc<dyn Any + Send + Sync>>::Watermark(w);
                        let _ = out_tx
                          .send(Arc::new(wm) as Arc<dyn Any + Send + Sync>)
                          .await;
                      }
                    }
                  }
                }
              } else {
                let _ = error_tx
                  .send(
                    Arc::new("Invalid key_function (expected GroupByConfigWrapper)".to_string())
                      as Arc<dyn Any + Send + Sync>,
                  )
                  .await;
              }
            }
            InputPort::In => {
              let Some(kf) = &key_fn else {
                buffer.push((InputPort::In, item));
                continue;
              };
              let Some(msg) = try_differential_message(&item) else {
                continue;
              };
              match msg {
                DifferentialStreamMessage::Data(elem) => {
                  let key = match kf.extract_key(elem.payload().clone()).await {
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
                  let count = state.entry(key.clone()).or_insert(0);
                  *count += diff;
                  let out_elem = DifferentialElement::new(
                    Arc::new((key, *count)) as Arc<dyn Any + Send + Sync>,
                    time,
                    diff,
                  );
                  let out_msg = Arc::new(DifferentialStreamMessage::Data(out_elem))
                    as Arc<dyn Any + Send + Sync>;
                  let _ = out_tx.send(out_msg).await;
                }
                DifferentialStreamMessage::Watermark(w) => {
                  let wm =
                    Arc::new(DifferentialStreamMessage::<Arc<dyn Any + Send + Sync>>::Watermark(w))
                      as Arc<dyn Any + Send + Sync>;
                  let _ = out_tx.send(wm).await;
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
