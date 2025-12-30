use bytes::Bytes;
use criterion::async_executor::FuturesExecutor;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::collections::HashMap;
use std::sync::Arc;
use streamweave_graph::NodeTrait;
use streamweave_graph::batching::{BatchBuffer, deserialize_batch, serialize_batch};
use streamweave_graph::channels::{ChannelItem, TypeErasedReceiver, TypeErasedSender};
use streamweave_graph::execution::BatchConfig;
use streamweave_graph::execution::ExecutionMode;
use streamweave_graph::node::ProducerNode;
use streamweave_graph::serialization::{JsonSerializer, deserialize, serialize};
use streamweave_vec::VecProducer;
use tokio::sync::{RwLock, mpsc};

/// Benchmark batch serialization/deserialization
fn batch_serialization_benchmark(c: &mut Criterion) {
  let mut group = c.benchmark_group("batch_serialization");

  for batch_size in [10, 100, 1000].iter() {
    // Create test data
    let items: Vec<Bytes> = (0..*batch_size).map(|i| serialize(&i).unwrap()).collect();

    group.throughput(Throughput::Elements(*batch_size as u64));

    group.bench_with_input(
      BenchmarkId::new("serialize", batch_size),
      &items,
      |b, items| {
        b.iter(|| serialize_batch(items.clone()));
      },
    );

    // Serialize once for deserialization benchmark
    let serialized = serialize_batch(items.clone()).unwrap();

    group.bench_with_input(
      BenchmarkId::new("deserialize", batch_size),
      &serialized,
      |b, bytes| {
        b.iter(|| deserialize_batch(bytes.clone()));
      },
    );
  }

  group.finish();
}

/// Benchmark BatchBuffer operations
fn batch_buffer_benchmark(c: &mut Criterion) {
  let mut group = c.benchmark_group("batch_buffer");

  let config = BatchConfig::new(100, 1000);
  let mut buffer = BatchBuffer::new(config);

  // Benchmark adding items
  group.bench_function("add_item", |b| {
    b.iter(|| {
      buffer.add(Bytes::from("test item")).unwrap();
    });
  });

  // Benchmark should_flush check
  group.bench_function("should_flush", |b| {
    b.iter(|| buffer.should_flush());
  });

  // Benchmark flush
  group.bench_function("flush", |b| {
    b.iter(|| {
      for i in 0..100 {
        buffer.add(Bytes::from(format!("item{}", i))).unwrap();
      }
      let items = buffer.flush();
      items.len()
    });
  });

  group.finish();
}

/// Benchmark producer with and without batching
async fn producer_with_batching(items: Vec<i32>, batching: Option<BatchConfig>) {
  let producer = VecProducer::new(items);
  let node: ProducerNode<_, (i32,)> = ProducerNode::new("producer".to_string(), producer);

  let (tx, mut rx): (TypeErasedSender, TypeErasedReceiver) = mpsc::channel(1000);
  let mut output_channels = HashMap::new();
  output_channels.insert(0, tx);
  let pause_signal = Arc::new(RwLock::new(false));
  let execution_mode = ExecutionMode::Distributed {
    serializer: JsonSerializer,
    compression: None,
    batching,
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
    if let ChannelItem::Bytes(bytes) = channel_item {
      if let Ok(batch_items) = deserialize_batch(bytes.clone()) {
        // Batched
        for item_bytes in batch_items {
          let _: i32 = deserialize(item_bytes).unwrap();
        }
      } else {
        // Not batched
        let _: i32 = deserialize(bytes).unwrap();
      }
    }
  }

  let _ = handle.await;
}

async fn producer_without_batching(items: Vec<i32>) {
  producer_with_batching(items, None).await
}

async fn producer_with_batching_small(items: Vec<i32>) {
  producer_with_batching(items, Some(BatchConfig::new(10, 100))).await
}

async fn producer_with_batching_large(items: Vec<i32>) {
  producer_with_batching(items, Some(BatchConfig::new(1000, 1000))).await
}

fn producer_batching_benchmark(c: &mut Criterion) {
  let mut group = c.benchmark_group("producer_batching");

  for size in [100, 1000, 10000].iter() {
    let items: Vec<i32> = (0..*size).collect();
    let throughput = Throughput::Elements(*size as u64);

    group.throughput(throughput);

    group.bench_with_input(BenchmarkId::new("no_batching", size), &items, |b, items| {
      b.to_async(FuturesExecutor)
        .iter(|| producer_without_batching(items.clone()));
    });

    group.bench_with_input(
      BenchmarkId::new("batching_small", size),
      &items,
      |b, items| {
        b.to_async(FuturesExecutor)
          .iter(|| producer_with_batching_small(items.clone()));
      },
    );

    group.bench_with_input(
      BenchmarkId::new("batching_large", size),
      &items,
      |b, items| {
        b.to_async(FuturesExecutor)
          .iter(|| producer_with_batching_large(items.clone()));
      },
    );
  }

  group.finish();
}

criterion_group!(
  benches,
  batch_serialization_benchmark,
  batch_buffer_benchmark,
  producer_batching_benchmark
);
criterion_main!(benches);
