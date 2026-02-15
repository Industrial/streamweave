//! # Bounded Iteration Node
//!
//! Runs an inner graph for up to `max_rounds` iterations. Round 1 uses seed input only;
//! rounds 2..N feed the previous round's output back as feedback.
//!
//! ## Ports
//!
//! - **Input**: `seed` - Initial input for round 1
//! - **Output**: `output` - Items produced each round (concatenated)
//! - **Output**: `converged` - Emits when inner graph signals early stop
//!
//! See [docs/cyclic-iterative-dataflows.md](../../../docs/cyclic-iterative-dataflows.md).

use crate::graph::Graph;
use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::BaseNode;
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

fn empty_stream() -> Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>> {
  Box::pin(stream::iter(vec![]))
}

/// Runs an inner graph for up to `max_rounds`. Round 1: seed only. Rounds 2..N: feedback from previous output.
pub struct BoundedIterationNode {
  base: BaseNode,
  inner_graph: Arc<Graph>,
  max_rounds: usize,
}

impl BoundedIterationNode {
  /// Creates a new BoundedIterationNode.
  ///
  /// The inner graph must have `seed` and `feedback` input ports, and `output` (and optionally `converged`) output ports.
  pub fn new(name: String, inner_graph: Graph, max_rounds: usize) -> Self {
    assert!(max_rounds >= 1, "max_rounds must be >= 1");
    Self {
      base: BaseNode::new(
        name,
        vec!["seed".to_string()],
        vec!["output".to_string(), "converged".to_string()],
      ),
      inner_graph: Arc::new(inner_graph),
      max_rounds,
    }
  }
}

#[async_trait]
impl Node for BoundedIterationNode {
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
    let inner_graph = Arc::clone(&self.inner_graph);
    let max_rounds = self.max_rounds;

    Box::pin(async move {
      let seed_stream = inputs.remove("seed").ok_or("Missing 'seed' input")?;
      let (out_tx, out_rx) = mpsc::channel(64);
      let (conv_tx, conv_rx) = mpsc::channel(10);

      tokio::spawn(async move {
        let mut feedback_buffer: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
        let mut seed = Some(seed_stream);

        for _round in 1..=max_rounds {
          let seed_stream = seed.take().unwrap_or_else(empty_stream);
          let feedback_stream: Pin<
            Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>,
          > = if feedback_buffer.is_empty() {
            empty_stream()
          } else {
            let buf = std::mem::take(&mut feedback_buffer);
            Box::pin(tokio_stream::iter(buf))
          };

          let mut inner_inputs = HashMap::new();
          inner_inputs.insert("seed".to_string(), seed_stream);
          inner_inputs.insert("feedback".to_string(), feedback_stream);

          let mut outputs = match inner_graph.execute(inner_inputs).await {
            Ok(o) => o,
            Err(e) => {
              let _ = conv_tx
                .send(Arc::new(format!("Inner graph error: {}", e)) as Arc<dyn Any + Send + Sync>)
                .await;
              break;
            }
          };

          let mut output_stream = match outputs.remove("output") {
            Some(s) => s,
            None => {
              let _ = conv_tx
                .send(Arc::new("Inner graph missing 'output' port".to_string())
                  as Arc<dyn Any + Send + Sync>)
                .await;
              break;
            }
          };

          let converged_stream = outputs.remove("converged");
          let mut converged_emitted = false;

          let out_tx_clone = out_tx.clone();
          let conv_tx_clone = conv_tx.clone();

          let output_task = tokio::spawn(async move {
            let mut buf = Vec::new();
            while let Some(item) = output_stream.next().await {
              buf.push(item.clone());
              let _ = out_tx_clone.send(item).await;
            }
            buf
          });

          if let Some(mut conv_stream) = converged_stream {
            while let Some(item) = conv_stream.next().await {
              converged_emitted = true;
              let _ = conv_tx_clone.send(item).await;
            }
          }

          feedback_buffer = output_task.await.unwrap_or_default();
          if converged_emitted {
            break;
          }
        }
      });

      let mut outputs = HashMap::new();
      outputs.insert(
        "output".to_string(),
        Box::pin(ReceiverStream::new(out_rx))
          as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );
      outputs.insert(
        "converged".to_string(),
        Box::pin(ReceiverStream::new(conv_rx))
          as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );
      Ok(outputs)
    })
  }
}
