use criterion::async_executor::FuturesExecutor;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::collections::HashMap;
use std::sync::Arc;
use streamweave_graph::NodeTrait;
use streamweave_graph::channels::{ChannelItem, TypeErasedReceiver, TypeErasedSender};
use streamweave_graph::execution::ExecutionMode;
use streamweave_graph::node::{ProducerNode, TransformerNode};
use streamweave_graph::serialization::{JsonSerializer, deserialize, serialize};
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecProducer;
use tokio::sync::{RwLock, mpsc};

/// Benchmark producer execution in distributed mode (serialized)
async fn producer_distributed(items: Vec<i32>) {
  let producer = VecProducer::new(items);
  let node: ProducerNode<_, (i32,)> = ProducerNode::new("producer".to_string(), producer);

  let (tx, mut rx): (TypeErasedSender, TypeErasedReceiver) = mpsc::channel(1000);
  let mut output_channels = HashMap::new();
  output_channels.insert(0, tx);
  let pause_signal = Arc::new(RwLock::new(false));
  let execution_mode = ExecutionMode::Distributed {
    serializer: JsonSerializer,
    compression: None,
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
    match channel_item {
      ChannelItem::Bytes(bytes) => {
        let _: i32 = deserialize(bytes).unwrap();
      }
      ChannelItem::Arc(_) => {
        panic!("Unexpected Arc in distributed mode");
      }
      ChannelItem::SharedMemory(_) => {
        panic!("Unexpected SharedMemory in distributed mode");
      }
    }
  }

  let _ = handle.await;
}

/// Benchmark producer execution in in-process mode (zero-copy)
async fn producer_in_process(items: Vec<i32>) {
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
    match channel_item {
      ChannelItem::Arc(arc) => {
        let typed_arc = Arc::downcast::<i32>(arc).unwrap();
        let _ = Arc::try_unwrap(typed_arc).unwrap_or_else(|arc| *arc);
      }
      ChannelItem::Bytes(_) => {
        panic!("Unexpected Bytes in in-process mode");
      }
      ChannelItem::SharedMemory(_) => {
        panic!("Unexpected SharedMemory in in-process mode without shared memory");
      }
    }
  }

  let _ = handle.await;
}

/// Benchmark transformer execution in distributed mode (serialized)
async fn transformer_distributed(items: Vec<i32>) {
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let node: TransformerNode<_, (i32,), (i32,)> =
    TransformerNode::new("transformer".to_string(), transformer);

  let (input_tx, input_rx): (TypeErasedSender, TypeErasedReceiver) = mpsc::channel(1000);
  let (output_tx, mut output_rx): (TypeErasedSender, TypeErasedReceiver) = mpsc::channel(1000);

  let mut input_channels = HashMap::new();
  input_channels.insert(0, input_rx);
  let mut output_channels = HashMap::new();
  output_channels.insert(0, output_tx);

  let pause_signal = Arc::new(RwLock::new(false));
  let execution_mode = ExecutionMode::Distributed {
    serializer: JsonSerializer,
    compression: None,
    batching: None,
  };

  let handle = node
    .spawn_execution_task(
      input_channels,
      output_channels,
      pause_signal,
      execution_mode,
      None,
      None,
    )
    .unwrap();

  // Send all items
  for item in items {
    let bytes = serialize(&item).unwrap();
    input_tx.send(ChannelItem::Bytes(bytes)).await.unwrap();
  }
  drop(input_tx);

  // Consume all outputs
  while let Some(channel_item) = output_rx.recv().await {
    match channel_item {
      ChannelItem::Bytes(bytes) => {
        let _: i32 = deserialize(bytes).unwrap();
      }
      ChannelItem::Arc(_) => {
        panic!("Unexpected Arc in distributed mode");
      }
      ChannelItem::SharedMemory(_) => {
        panic!("Unexpected SharedMemory in distributed mode");
      }
    }
  }

  let _ = handle.await;
}

/// Benchmark transformer execution in in-process mode (zero-copy)
async fn transformer_in_process(items: Vec<i32>) {
  let transformer = MapTransformer::new(|x: i32| x * 2);
  let node: TransformerNode<_, (i32,), (i32,)> =
    TransformerNode::new("transformer".to_string(), transformer);

  let (input_tx, input_rx): (TypeErasedSender, TypeErasedReceiver) = mpsc::channel(1000);
  let (output_tx, mut output_rx): (TypeErasedSender, TypeErasedReceiver) = mpsc::channel(1000);

  let mut input_channels = HashMap::new();
  input_channels.insert(0, input_rx);
  let mut output_channels = HashMap::new();
  output_channels.insert(0, output_tx);

  let pause_signal = Arc::new(RwLock::new(false));
  let execution_mode = ExecutionMode::InProcess {
    use_shared_memory: false,
  };

  let handle = node
    .spawn_execution_task(
      input_channels,
      output_channels,
      pause_signal,
      execution_mode,
      None,
      None,
    )
    .unwrap();

  // Send all items as Arc
  for item in items {
    input_tx
      .send(ChannelItem::Arc(Arc::new(item)))
      .await
      .unwrap();
  }
  drop(input_tx);

  // Consume all outputs
  while let Some(channel_item) = output_rx.recv().await {
    match channel_item {
      ChannelItem::Arc(arc) => {
        let typed_arc = Arc::downcast::<i32>(arc).unwrap();
        let _ = Arc::try_unwrap(typed_arc).unwrap_or_else(|arc| *arc);
      }
      ChannelItem::Bytes(_) => {
        panic!("Unexpected Bytes in in-process mode");
      }
      ChannelItem::SharedMemory(_) => {
        panic!("Unexpected SharedMemory in in-process mode without shared memory");
      }
    }
  }

  let _ = handle.await;
}

/// Benchmark fan-out in distributed mode
async fn fan_out_distributed(items: Vec<i32>) {
  let producer = VecProducer::new(items);
  let node: ProducerNode<_, (i32,)> = ProducerNode::new("producer".to_string(), producer);

  let (tx1, mut rx1): (TypeErasedSender, TypeErasedReceiver) = mpsc::channel(1000);
  let (tx2, mut rx2): (TypeErasedSender, TypeErasedReceiver) = mpsc::channel(1000);
  let mut output_channels = HashMap::new();
  output_channels.insert(0, tx1);
  output_channels.insert(1, tx2);
  let pause_signal = Arc::new(RwLock::new(false));
  let execution_mode = ExecutionMode::Distributed {
    serializer: JsonSerializer,
    compression: None,
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

  // Consume from both receivers
  let mut _count = 0;
  loop {
    tokio::select! {
      item1 = rx1.recv() => {
        match item1 {
          Some(ChannelItem::Bytes(bytes)) => {
            let _: i32 = deserialize(bytes).unwrap();
            _count += 1;
          }
          Some(ChannelItem::Arc(_)) | Some(ChannelItem::SharedMemory(_)) => {
            panic!("Unexpected variant in distributed mode");
          }
          None => break,
        }
      }
      item2 = rx2.recv() => {
        match item2 {
          Some(ChannelItem::Bytes(bytes)) => {
            let _: i32 = deserialize(bytes).unwrap();
            _count += 1;
          }
          Some(ChannelItem::Arc(_)) | Some(ChannelItem::SharedMemory(_)) => {
            panic!("Unexpected variant in distributed mode");
          }
          None => break,
        }
      }
    }
  }

  let _ = handle.await;
}

/// Benchmark fan-out in in-process mode (zero-copy)
async fn fan_out_in_process(items: Vec<i32>) {
  let producer = VecProducer::new(items);
  let node: ProducerNode<_, (i32,)> = ProducerNode::new("producer".to_string(), producer);

  let (tx1, mut rx1): (TypeErasedSender, TypeErasedReceiver) = mpsc::channel(1000);
  let (tx2, mut rx2): (TypeErasedSender, TypeErasedReceiver) = mpsc::channel(1000);
  let mut output_channels = HashMap::new();
  output_channels.insert(0, tx1);
  output_channels.insert(1, tx2);
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

  // Consume from both receivers
  let mut _count = 0;
  loop {
    tokio::select! {
      item1 = rx1.recv() => {
        match item1 {
          Some(ChannelItem::Arc(arc)) => {
            let typed_arc = Arc::downcast::<i32>(arc).unwrap();
            let _ = Arc::try_unwrap(typed_arc).unwrap_or_else(|arc| *arc);
            _count += 1;
          }
          Some(ChannelItem::Bytes(_)) | Some(ChannelItem::SharedMemory(_)) => {
            panic!("Unexpected variant in in-process mode");
          }
          None => break,
        }
      }
      item2 = rx2.recv() => {
        match item2 {
          Some(ChannelItem::Arc(arc)) => {
            let typed_arc = Arc::downcast::<i32>(arc).unwrap();
            let _ = Arc::try_unwrap(typed_arc).unwrap_or_else(|arc| *arc);
            _count += 1;
          }
          Some(ChannelItem::Bytes(_)) | Some(ChannelItem::SharedMemory(_)) => {
            panic!("Unexpected variant in in-process mode");
          }
          None => break,
        }
      }
    }
  }

  let _ = handle.await;
}

fn producer_benchmark(c: &mut Criterion) {
  let mut group = c.benchmark_group("producer");

  for size in [100, 1000, 10000].iter() {
    let items: Vec<i32> = (0..*size).collect();
    let throughput = Throughput::Elements(*size as u64);

    group.throughput(throughput);
    group.bench_with_input(BenchmarkId::new("distributed", size), &items, |b, items| {
      b.to_async(FuturesExecutor)
        .iter(|| producer_distributed(items.clone()));
    });

    group.bench_with_input(BenchmarkId::new("in_process", size), &items, |b, items| {
      b.to_async(FuturesExecutor)
        .iter(|| producer_in_process(items.clone()));
    });
  }

  group.finish();
}

fn transformer_benchmark(c: &mut Criterion) {
  let mut group = c.benchmark_group("transformer");

  for size in [100, 1000, 10000].iter() {
    let items: Vec<i32> = (0..*size).collect();
    let throughput = Throughput::Elements(*size as u64);

    group.throughput(throughput);
    group.bench_with_input(BenchmarkId::new("distributed", size), &items, |b, items| {
      b.to_async(FuturesExecutor)
        .iter(|| transformer_distributed(items.clone()));
    });

    group.bench_with_input(BenchmarkId::new("in_process", size), &items, |b, items| {
      b.to_async(FuturesExecutor)
        .iter(|| transformer_in_process(items.clone()));
    });
  }

  group.finish();
}

fn fan_out_benchmark(c: &mut Criterion) {
  let mut group = c.benchmark_group("fan_out");

  for size in [100, 1000, 10000].iter() {
    let items: Vec<i32> = (0..*size).collect();
    let throughput = Throughput::Elements(*size as u64 * 2); // 2 outputs

    group.throughput(throughput);
    group.bench_with_input(BenchmarkId::new("distributed", size), &items, |b, items| {
      b.to_async(FuturesExecutor)
        .iter(|| fan_out_distributed(items.clone()));
    });

    group.bench_with_input(BenchmarkId::new("in_process", size), &items, |b, items| {
      b.to_async(FuturesExecutor)
        .iter(|| fan_out_in_process(items.clone()));
    });
  }

  group.finish();
}

criterion_group!(
  benches,
  producer_benchmark,
  transformer_benchmark,
  fan_out_benchmark
);
criterion_main!(benches);
