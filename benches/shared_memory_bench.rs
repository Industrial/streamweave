//! # Shared Memory Benchmark
//!
//! **DISABLED**: This benchmark references old modules that have been removed:
//! - `streamweave::graph::channels`
//! - `streamweave::graph::nodes::ProducerNode`
//! - `streamweave::graph::traits`
//! - `streamweave::graph::serialization`
//! - `streamweave::graph::shared_memory_channel`
//!
//! This benchmark needs to be rewritten to use the new stream-based architecture.

fn main() {
  // Benchmark disabled - see comment above
}

/*
use criterion::async_executor::AsyncExecutor;

/// Tokio executor for criterion benchmarks
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
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::collections::HashMap;
use std::sync::Arc;
use streamweave::graph::channels::{ChannelItem, TypeErasedReceiver, TypeErasedSender};
use streamweave::graph::nodes::ProducerNode;
use streamweave::graph::serialization::serialize;
use streamweave::graph::shared_memory_channel::SharedMemoryChannel;
use streamweave::graph::traits::NodeTrait;
use streamweave::producers::VecProducer;
use tokio::sync::{RwLock, mpsc};

/// Benchmark producer execution with shared memory
async fn producer_shared_memory(items: Vec<i32>) {
  let producer = VecProducer::new(items);
  let node: ProducerNode<_, (i32,)> =
    ProducerNode::new("producer".to_string(), producer, vec!["out".to_string()]);

  let (tx, mut rx): (TypeErasedSender, TypeErasedReceiver) = mpsc::channel(1000);
  let mut output_channels = HashMap::new();
  output_channels.insert("out".to_string(), tx);
  let pause_signal = Arc::new(RwLock::new(false));
  let use_shared_memory = true;

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
  while let Some(channel_item) = rx.recv().await {
    if let ChannelItem::SharedMemory(_shared_mem_ref) = channel_item {
      // In a real implementation, we would read from shared memory here
      // For benchmarking, we just count items
    }
  }

  let _ = handle.await;
}

/// Benchmark producer execution with Arc<T> (zero-copy)
async fn producer_arc(items: Vec<i32>) {
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
  while let Some(channel_item) = rx.recv().await {
    if let ChannelItem::Arc(arc) = channel_item {
      let typed_arc = Arc::downcast::<i32>(arc).unwrap();
      let _ = Arc::try_unwrap(typed_arc).unwrap_or_else(|arc| *arc);
    }
  }

  let _ = handle.await;
}

/// Benchmark direct shared memory channel operations
async fn shared_memory_direct(items: Vec<i32>) {
  let channel = SharedMemoryChannel::new("bench_direct", 1024 * 1024).unwrap();

  // Send all items
  for item in &items {
    let bytes = serialize(item).unwrap();
    let _ = channel.send(&bytes).unwrap();
  }

  // Receive all items
  for _ in 0..items.len() {
    let _ = channel.receive().unwrap();
  }
}

/// Benchmark Arc channel operations (simulated)
async fn arc_channel_direct(items: Vec<i32>) {
  let (tx, mut rx): (mpsc::Sender<Arc<i32>>, mpsc::Receiver<Arc<i32>>) = mpsc::channel(1000);

  // Send all items
  for item in &items {
    tx.send(Arc::new(*item)).await.unwrap();
  }
  drop(tx);

  // Receive all items
  while let Some(arc) = rx.recv().await {
    let _ = Arc::try_unwrap(arc).unwrap_or_else(|arc| *arc);
  }
}

fn shared_memory_benchmark(c: &mut Criterion) {
  let mut group = c.benchmark_group("shared_memory");

  for size in [100, 1000, 10000].iter() {
    let items: Vec<i32> = (0..*size).collect();
    let throughput = Throughput::Elements(*size as u64);

    group.throughput(throughput);

    group.bench_with_input(
      BenchmarkId::new("shared_memory", size),
      &items,
      |b, items| {
        b.to_async(TokioExecutor)
          .iter(|| shared_memory_direct(items.clone()));
      },
    );

    group.bench_with_input(BenchmarkId::new("arc_channel", size), &items, |b, items| {
      b.to_async(TokioExecutor)
        .iter(|| arc_channel_direct(items.clone()));
    });
  }

  group.finish();
}

fn producer_comparison_benchmark(c: &mut Criterion) {
  let mut group = c.benchmark_group("producer_comparison");

  for size in [100, 1000, 10000].iter() {
    let items: Vec<i32> = (0..*size).collect();
    let throughput = Throughput::Elements(*size as u64);

    group.throughput(throughput);

    group.bench_with_input(
      BenchmarkId::new("shared_memory", size),
      &items,
      |b, items| {
        b.to_async(TokioExecutor)
          .iter(|| producer_shared_memory(items.clone()));
      },
    );

    group.bench_with_input(
      BenchmarkId::new("arc_zero_copy", size),
      &items,
      |b, items| {
        b.to_async(TokioExecutor)
          .iter(|| producer_arc(items.clone()));
      },
    );
  }

  group.finish();
}

criterion_group!(
  benches,
  shared_memory_benchmark,
  producer_comparison_benchmark
);
criterion_main!(benches);
*/
