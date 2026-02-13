//! # Graph Pass-Through Benchmark
//!
//! Benchmarks a graph with a single pass-through node (MapNode with identity function).
//! Measures throughput of data flowing through the graph pipeline.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::map_node::{MapNode, map_config};
use tokio::sync::mpsc;

/// Runs a graph with one pass-through node, sending `count` items through and consuming them.
/// Uses current_thread runtime to match #[tokio::test] behavior.
fn graph_pass_through_sync(count: usize) {
  let rt = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .unwrap();
  rt.block_on(graph_pass_through(count));
}

/// Async implementation: graph with one pass-through node.
async fn graph_pass_through(count: usize) {
  let identity_config = map_config(|value| async move { Ok(value) });

  let mut graph: Graph = graph! {
    map: MapNode::new("map".to_string()),
    graph.configuration => map.configuration,
    graph.input => map.in,
    map.out => graph.output
  };

  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(count + 1);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(count + 1);

  graph
    .connect_input_channel("configuration", config_rx)
    .unwrap();
  graph.connect_input_channel("input", input_rx).unwrap();
  graph.connect_output_channel("output", output_tx).unwrap();

  // Send config first (required before MapNode can process)
  let _ = config_tx
    .send(Arc::new(identity_config) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(config_tx);

  // Send all data before execute (matching map_node example; buffer sized to avoid blocking)
  for i in 0..count {
    let _ = input_tx
      .send(Arc::new(i as i32) as Arc<dyn Any + Send + Sync>)
      .await;
  }
  drop(input_tx);

  graph.execute().await.unwrap();

  let mut received = 0;
  while output_rx.recv().await.is_some() {
    received += 1;
    if received >= count {
      break;
    }
  }
  assert_eq!(received, count);
}

fn graph_pass_through_benchmark(c: &mut Criterion) {
  let mut group = c.benchmark_group("graph_pass_through");
  group.sample_size(10);
  group.warm_up_time(std::time::Duration::from_secs(1));
  group.measurement_time(std::time::Duration::from_secs(3));

  for size in [100, 1000, 10000].iter() {
    group.throughput(Throughput::Elements(*size as u64));
    group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
      b.iter(|| graph_pass_through_sync(size));
    });
  }

  group.finish();
}

criterion_group!(benches, graph_pass_through_benchmark);
criterion_main!(benches);
