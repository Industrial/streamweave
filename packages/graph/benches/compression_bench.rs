use bytes::Bytes;
use criterion::async_executor::FuturesExecutor;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::collections::HashMap;
use std::sync::Arc;
use streamweave_graph::NodeTrait;
use streamweave_graph::channels::{ChannelItem, TypeErasedReceiver, TypeErasedSender};
use streamweave_graph::compression::{Compression, GzipCompression, ZstdCompression};
use streamweave_graph::execution::{CompressionAlgorithm, ExecutionMode};
use streamweave_graph::node::ProducerNode;
use streamweave_graph::serialization::{JsonSerializer, deserialize, serialize};
use streamweave_vec::VecProducer;
use tokio::sync::{RwLock, mpsc};

/// Benchmark compression algorithms directly
fn compression_benchmark(c: &mut Criterion) {
  let mut group = c.benchmark_group("compression");

  // Test data: JSON-serialized integers
  let data: Vec<i32> = (0..1000).collect();
  let serialized: Vec<Bytes> = data.iter().map(|&x| serialize(&x).unwrap()).collect();

  let total_size: usize = serialized.iter().map(|s| s.len()).sum();
  group.throughput(criterion::Throughput::Bytes(total_size as u64));

  // Benchmark gzip compression
  group.bench_function("gzip_level_1", |b| {
    let compressor = GzipCompression::new(1);
    b.iter(|| {
      for bytes in &serialized {
        let _ = compressor.compress(bytes);
      }
    });
  });

  group.bench_function("gzip_level_6", |b| {
    let compressor = GzipCompression::new(6);
    b.iter(|| {
      for bytes in &serialized {
        let _ = compressor.compress(bytes);
      }
    });
  });

  group.bench_function("gzip_level_9", |b| {
    let compressor = GzipCompression::new(9);
    b.iter(|| {
      for bytes in &serialized {
        let _ = compressor.compress(bytes);
      }
    });
  });

  // Benchmark zstd compression
  group.bench_function("zstd_level_1", |b| {
    let compressor = ZstdCompression::new(1);
    b.iter(|| {
      for bytes in &serialized {
        let _ = compressor.compress(bytes);
      }
    });
  });

  group.bench_function("zstd_level_3", |b| {
    let compressor = ZstdCompression::new(3);
    b.iter(|| {
      for bytes in &serialized {
        let _ = compressor.compress(bytes);
      }
    });
  });

  group.bench_function("zstd_level_10", |b| {
    let compressor = ZstdCompression::new(10);
    b.iter(|| {
      for bytes in &serialized {
        let _ = compressor.compress(bytes);
      }
    });
  });

  group.finish();
}

/// Benchmark producer with and without compression
async fn producer_with_compression(items: Vec<i32>, compression: Option<CompressionAlgorithm>) {
  let producer = VecProducer::new(items);
  let node: ProducerNode<_, (i32,)> = ProducerNode::new("producer".to_string(), producer);

  let (tx, mut rx): (TypeErasedSender, TypeErasedReceiver) = mpsc::channel(1000);
  let mut output_channels = HashMap::new();
  output_channels.insert(0, tx);
  let pause_signal = Arc::new(RwLock::new(false));
  let execution_mode = ExecutionMode::Distributed {
    serializer: JsonSerializer,
    compression,
    batching: None,
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
      // In a real scenario, StreamWrapper would handle decompression
      // For benchmarking, we just count items
      let _: i32 = deserialize(bytes).unwrap();
    }
  }

  let _ = handle.await;
}

async fn producer_without_compression(items: Vec<i32>) {
  producer_with_compression(items, None).await
}

async fn producer_with_gzip(items: Vec<i32>) {
  producer_with_compression(items, Some(CompressionAlgorithm::Gzip { level: 6 })).await
}

async fn producer_with_zstd(items: Vec<i32>) {
  producer_with_compression(items, Some(CompressionAlgorithm::Zstd { level: 3 })).await
}

fn producer_compression_benchmark(c: &mut Criterion) {
  let mut group = c.benchmark_group("producer_compression");

  for size in [100, 1000, 10000].iter() {
    let items: Vec<i32> = (0..*size).collect();
    let throughput = Throughput::Elements(*size as u64);

    group.throughput(throughput);

    group.bench_with_input(
      BenchmarkId::new("no_compression", size),
      &items,
      |b, items| {
        b.to_async(FuturesExecutor)
          .iter(|| producer_without_compression(items.clone()));
      },
    );

    group.bench_with_input(
      BenchmarkId::new("gzip_level_6", size),
      &items,
      |b, items| {
        b.to_async(FuturesExecutor)
          .iter(|| producer_with_gzip(items.clone()));
      },
    );

    group.bench_with_input(
      BenchmarkId::new("zstd_level_3", size),
      &items,
      |b, items| {
        b.to_async(FuturesExecutor)
          .iter(|| producer_with_zstd(items.clone()));
      },
    );
  }

  group.finish();
}

criterion_group!(
  benches,
  compression_benchmark,
  producer_compression_benchmark
);
criterion_main!(benches);
