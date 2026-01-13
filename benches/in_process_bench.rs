//! # In-Process Benchmark
//!
//! **DISABLED**: This benchmark references old modules that have been removed:
//! - `streamweave::graph::channels`
//! - `streamweave::graph::nodes::ProducerNode`
//! - `streamweave::graph::traits`
//!
//! This benchmark needs to be rewritten to use the new stream-based architecture.

fn main() {
  // Benchmark disabled - see comment above
}

/*
use criterion::async_executor::AsyncExecutor;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::collections::HashMap;
use std::sync::Arc;
use streamweave::graph::channels::{ChannelItem, TypeErasedReceiver, TypeErasedSender};
use streamweave::graph::nodes::ProducerNode;
use streamweave::graph::traits::NodeTrait;
use streamweave::producers::VecProducer;
use tokio::sync::{RwLock, mpsc};

/// Tokio executor for criterion benchmarks
#[derive(Clone, Copy)]
struct TokioExecutor;

impl AsyncExecutor for TokioExecutor {
  fn block_on<T>(&self, future: impl std::future::Future<Output = T>) -> T {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(future)
  }
}

/// Benchmark producer execution in in-process mode
async fn producer_in_process(items: Vec<i32>) {
  let producer = VecProducer::new(items);
  let node: ProducerNode<_, (i32,)> =
    ProducerNode::new("producer".to_string(), producer, vec!["out".to_string()]);

  let (tx, mut rx): (TypeErasedSender, TypeErasedReceiver) = mpsc::channel(1000);
  let mut output_channels = HashMap::new();
  output_channels.insert("out".to_string(), tx);
  let pause_signal = Arc::new(RwLock::new(false));
  let use_shared_memory = false;

  let handle = node
    .spawn_execution_task(
      HashMap::new(),
      output_channels,
      pause_signal,
      use_shared_memory,
      None,
    )
    .unwrap();

  // Consume all items
  let mut count = 0;
  while let Some(channel_item) = rx.recv().await {
    match channel_item {
      ChannelItem::Arc(_) => {
        // Downcast to Arc<Message<i32>> and consume
        let _msg_arc = channel_item.downcast_message_arc::<i32>().unwrap();
        count += 1;
      }
      ChannelItem::SharedMemory(_) => {
        // Shared memory variant (shouldn't happen with use_shared_memory: false)
        count += 1;
      }
    }
  }

  let _ = handle.await;
  assert!(count > 0, "Should have received at least one item");
}

fn producer_benchmark(c: &mut Criterion) {
  let mut group = c.benchmark_group("producer_in_process");

  for size in [100, 1000, 10000].iter() {
    let items: Vec<i32> = (0..*size).collect();
    group.throughput(Throughput::Elements(*size as u64));
    group.bench_with_input(BenchmarkId::from_parameter(size), &items, |b, items| {
      b.to_async(TokioExecutor)
        .iter(|| producer_in_process(items.clone()));
    });
  }

  group.finish();
}

criterion_group!(benches, producer_benchmark);
criterion_main!(benches);
*/
