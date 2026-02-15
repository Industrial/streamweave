//! # Timestamped differential dataflow
//!
//! Demonstrates [`ToDifferentialNode`], [`DifferentialGroupByNode`], and [`DifferentialJoinNode`]:
//! wrap a stream as (payload, time, diff), incremental group-by count, and differential equi-join.
//!
//! See [docs/timestamped-differential-dataflow.md](docs/timestamped-differential-dataflow.md) and
//! [docs/EXAMPLES-AND-HOW-TO.md](docs/EXAMPLES-AND-HOW-TO.md).

use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::edge::Edge;
use streamweave::graph::Graph;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use streamweave::nodes::differential_join_node::DifferentialJoinNode;
use streamweave::nodes::join_node::{join_config, JoinStrategy};
use streamweave::nodes::reduction::{
    group_by_config, DifferentialGroupByNode, GroupByConfigWrapper,
};
use streamweave::nodes::stream::ToDifferentialNode;
use streamweave::time::DifferentialStreamMessage;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

/// Producer that emits a sequence of (key, value) pairs as `Arc<(String, i64)>`.
struct ProducerNode {
    name: String,
    data: Vec<(String, i64)>,
    output_port_names: Vec<String>,
}

impl ProducerNode {
    fn new(name: String, data: Vec<(String, i64)>) -> Self {
        Self {
            name,
            data,
            output_port_names: vec!["out".to_string()],
        }
    }
}

#[async_trait]
impl Node for ProducerNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }
    fn input_port_names(&self) -> &[String] {
        &[]
    }
    fn output_port_names(&self) -> &[String] {
        &self.output_port_names
    }
    fn has_input_port(&self, _name: &str) -> bool {
        false
    }
    fn has_output_port(&self, name: &str) -> bool {
        name == "out"
    }
    fn execute(
        &self,
        _inputs: InputStreams,
    ) -> Pin<
        Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>,
    > {
        let data = self.data.clone();
        Box::pin(async move {
            let (tx, rx) = mpsc::channel(10);
            tokio::spawn(async move {
                for kv in data {
                    let _ = tx
                        .send(Arc::new(kv) as Arc<dyn Any + Send + Sync>)
                        .await;
                }
            });
            let mut outputs = HashMap::new();
            outputs.insert(
                "out".to_string(),
                Box::pin(ReceiverStream::new(rx))
                    as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
            );
            Ok(outputs)
        })
    }
}

/// Second producer for join demo: emits HashMap items with "id" and "name" or "val".
struct HashMapProducerNode {
    name: String,
    data: Vec<HashMap<String, i32>>,
    output_port_names: Vec<String>,
}

impl HashMapProducerNode {
    fn new(name: String, data: Vec<HashMap<String, i32>>) -> Self {
        Self {
            name,
            data,
            output_port_names: vec!["out".to_string()],
        }
    }
}

#[async_trait]
impl Node for HashMapProducerNode {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }
    fn input_port_names(&self) -> &[String] {
        &[]
    }
    fn output_port_names(&self) -> &[String] {
        &self.output_port_names
    }
    fn has_input_port(&self, _name: &str) -> bool {
        false
    }
    fn has_output_port(&self, name: &str) -> bool {
        name == "out"
    }
    fn execute(
        &self,
        _inputs: InputStreams,
    ) -> Pin<
        Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>,
    > {
        let data = self.data.clone();
        Box::pin(async move {
            let (tx, rx) = mpsc::channel(10);
            tokio::spawn(async move {
                for map in data {
                    let _ = tx
                        .send(Arc::new(map) as Arc<dyn Any + Send + Sync>)
                        .await;
                }
            });
            let mut outputs = HashMap::new();
            outputs.insert(
                "out".to_string(),
                Box::pin(ReceiverStream::new(rx))
                    as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
            );
            Ok(outputs)
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(run())
}

async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Differential stream example (ToDifferentialNode, DifferentialGroupBy, DifferentialJoin)\n");

    // --- Part 1: ToDifferential + DifferentialGroupBy ---
    println!("Part 1: ToDifferentialNode -> DifferentialGroupByNode (count per key)");
    let mut graph = Graph::new("differential_group_by".to_string());
    graph
        .add_node(
            "producer".to_string(),
            Box::new(ProducerNode::new(
                "producer".to_string(),
                vec![
                    ("a".to_string(), 1),
                    ("b".to_string(), 1),
                    ("a".to_string(), 2),
                ],
            )),
        )
        .unwrap();
    graph
        .add_node(
            "to_diff".to_string(),
            Box::new(ToDifferentialNode::new("to_diff".to_string())),
        )
        .unwrap();
    graph
        .add_node(
            "diff_group_by".to_string(),
            Box::new(DifferentialGroupByNode::new("diff_group_by".to_string())),
        )
        .unwrap();

    graph
        .add_edge(Edge {
            source_node: "producer".to_string(),
            source_port: "out".to_string(),
            target_node: "to_diff".to_string(),
            target_port: "in".to_string(),
        })
        .unwrap();
    graph
        .add_edge(Edge {
            source_node: "to_diff".to_string(),
            source_port: "out".to_string(),
            target_node: "diff_group_by".to_string(),
            target_port: "in".to_string(),
        })
        .unwrap();

    // Key: extract first element from (String, i64)
    let key_config = group_by_config(|value| {
        Box::pin(async move {
            let arc = value
                .downcast::<(String, i64)>()
                .map_err(|_| "Expected (String, i64)".to_string())?;
            Ok(arc.0.clone())
        })
    });
    graph
        .expose_input_port("diff_group_by", "key_function", "key_config")
        .unwrap();
    graph
        .expose_output_port("diff_group_by", "out", "output")
        .unwrap();

    let (key_tx, key_rx) = mpsc::channel(1);
    key_tx
        .send(Arc::new(GroupByConfigWrapper(key_config)) as Arc<dyn Any + Send + Sync>)
        .await?;
    drop(key_tx);
    graph.connect_input_channel("key_config", key_rx).unwrap();

    let (out_tx, mut out_rx) = mpsc::channel(10);
    graph.connect_output_channel("output", out_tx).unwrap();

    Graph::execute(&mut graph).await?;
    let mut group_by_results: Vec<(String, i64, u64, i64)> = Vec::new();
    while let Some(arc) = out_rx.recv().await {
        if let Ok(msg) = arc.clone().downcast::<DifferentialStreamMessage<Arc<dyn Any + Send + Sync>>>() {
            if let DifferentialStreamMessage::Data(elem) = msg.as_ref() {
                if let Ok(pair) = elem.payload().clone().downcast::<(String, i64)>() {
                    group_by_results.push((pair.0.clone(), pair.1, elem.time().as_u64(), elem.diff()));
                }
            }
        }
    }
    graph.wait_for_completion().await?;
    println!("  Group-by output (key, count, time, diff): {:?}", group_by_results);
    println!();

    // --- Part 2: ToDifferential + DifferentialJoin ---
    println!("Part 2: Two sources -> ToDifferential -> DifferentialJoinNode (inner join on id)");
    let mut graph2 = Graph::new("differential_join".to_string());
    let mut left_data = HashMap::new();
    left_data.insert("id".to_string(), 1);
    left_data.insert("name".to_string(), 10);
    let mut right_data = HashMap::new();
    right_data.insert("id".to_string(), 1);
    right_data.insert("val".to_string(), 100);

    graph2
        .add_node(
            "left_producer".to_string(),
            Box::new(HashMapProducerNode::new(
                "left_producer".to_string(),
                vec![left_data],
            )),
        )
        .unwrap();
    graph2
        .add_node(
            "right_producer".to_string(),
            Box::new(HashMapProducerNode::new(
                "right_producer".to_string(),
                vec![right_data],
            )),
        )
        .unwrap();
    graph2
        .add_node(
            "to_diff_left".to_string(),
            Box::new(ToDifferentialNode::new("to_diff_left".to_string())),
        )
        .unwrap();
    graph2
        .add_node(
            "to_diff_right".to_string(),
            Box::new(ToDifferentialNode::new("to_diff_right".to_string())),
        )
        .unwrap();
    graph2
        .add_node(
            "diff_join".to_string(),
            Box::new(DifferentialJoinNode::new("diff_join".to_string())),
        )
        .unwrap();

    graph2
        .add_edge(Edge {
            source_node: "left_producer".to_string(),
            source_port: "out".to_string(),
            target_node: "to_diff_left".to_string(),
            target_port: "in".to_string(),
        })
        .unwrap();
    graph2
        .add_edge(Edge {
            source_node: "right_producer".to_string(),
            source_port: "out".to_string(),
            target_node: "to_diff_right".to_string(),
            target_port: "in".to_string(),
        })
        .unwrap();
    graph2
        .add_edge(Edge {
            source_node: "to_diff_left".to_string(),
            source_port: "out".to_string(),
            target_node: "diff_join".to_string(),
            target_port: "left".to_string(),
        })
        .unwrap();
    graph2
        .add_edge(Edge {
            source_node: "to_diff_right".to_string(),
            source_port: "out".to_string(),
            target_node: "diff_join".to_string(),
            target_port: "right".to_string(),
        })
        .unwrap();

    let id_key_fn = |item: Arc<dyn Any + Send + Sync>| {
        Box::pin(async move {
            if let Ok(arc) = item.downcast::<HashMap<String, i32>>() {
                if let Some(id) = arc.get("id") {
                    return Ok(id.to_string());
                }
            }
            Err("Missing or invalid 'id'".to_string())
        })
    };
    let join_cfg = join_config(
        JoinStrategy::Inner,
        id_key_fn,
        id_key_fn,
        |left: Arc<dyn Any + Send + Sync>, right: Option<Arc<dyn Any + Send + Sync>>| {
            Box::pin(async move {
                let mut result = HashMap::new();
                result.insert("left".to_string(), left);
                if let Some(r) = right {
                    result.insert("right".to_string(), r);
                }
                Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
            })
        },
    );
    graph2
        .expose_input_port("diff_join", "configuration", "join_config")
        .unwrap();
    graph2
        .expose_output_port("diff_join", "out", "output")
        .unwrap();

    let (cfg_tx, cfg_rx) = mpsc::channel(1);
    cfg_tx
        .send(Arc::new(join_cfg) as Arc<dyn Any + Send + Sync>)
        .await?;
    drop(cfg_tx);
    graph2.connect_input_channel("join_config", cfg_rx).unwrap();

    let (out_tx2, mut out_rx2) = mpsc::channel(10);
    graph2.connect_output_channel("output", out_tx2).unwrap();

    Graph::execute(&mut graph2).await?;
    let mut join_results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
    while let Some(arc) = out_rx2.recv().await {
        if let Ok(msg) = arc.clone().downcast::<DifferentialStreamMessage<Arc<dyn Any + Send + Sync>>>() {
            if let DifferentialStreamMessage::Data(elem) = msg.as_ref() {
                join_results.push(elem.payload().clone());
            }
        }
    }
    graph2.wait_for_completion().await?;
    println!("  Join output count: {} (payloads are HashMap with left/right)", join_results.len());
    println!();

    println!("Done.");
    Ok(())
}
