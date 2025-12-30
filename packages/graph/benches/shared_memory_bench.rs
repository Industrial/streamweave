use criterion::async_executor::FuturesExecutor;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::collections::HashMap;
use std::sync::Arc;
use streamweave_graph::NodeTrait;
use streamweave_graph::channels::{ChannelItem, TypeErasedReceiver, TypeErasedSender};
use streamweave_graph::execution::ExecutionMode;
use streamweave_graph::node::ProducerNode;
use streamweave_graph::serialization::serialize;
use streamweave_graph::shared_memory_channel::SharedMemoryChannel;
use streamweave_vec::VecProducer;
use tokio::sync::{RwLock, mpsc};

/// Benchmark producer execution with shared memory
async fn producer_shared_memory(items: Vec<i32>) {
  let producer = VecProducer::new(items);
  let node: ProducerNode<_, (i32,)> = ProducerNode::new("producer".to_string(), producer);

  let (tx, mut rx): (TypeErasedSender, TypeErasedReceiver) = mpsc::channel(1000);
  let mut output_channels = HashMap::new();
  output_channels.insert(0, tx);
  let pause_signal = Arc::new(RwLock::new(false));
  let execution_mode = ExecutionMode::InProcess {
    use_shared_memory: true,
  };

  let handle = node
    .spawn_execution_task(
      HashMap::new(),
      output_channels,
      pause_signal,
      execution_mode,
      None,
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
  let node: ProducerNode<_, (i32,)> = ProducerNode::new("producer".to_string(), producer);

  let (tx, mut rx): (TypeErasedSender, TypeErasedReceiver) = mpsc::channel(1000);
  let mut output_channels = HashMap::new();
  output_channels.insert(0, tx);
  let pause_signal = Arc::new(RwLock::new(false));
  let execution_mode = ExecutionMode::InProcess {
    use_shared_memory: false,
  };

  let handle = node
    .spawn_execution_task(
      HashMap::new(),
      output_channels,
      pause_signal,
      execution_mode,
      None,
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
        b.to_async(FuturesExecutor)
          .iter(|| shared_memory_direct(items.clone()));
      },
    );

    group.bench_with_input(BenchmarkId::new("arc_channel", size), &items, |b, items| {
      b.to_async(FuturesExecutor)
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
        b.to_async(FuturesExecutor)
          .iter(|| producer_shared_memory(items.clone()));
      },
    );

    group.bench_with_input(
      BenchmarkId::new("arc_zero_copy", size),
      &items,
      |b, items| {
        b.to_async(FuturesExecutor)
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
