use crate::parquet_producer::ParquetProducer;
use arrow::record_batch::RecordBatch;
use futures::Stream;
use std::pin::Pin;
use streamweave_core::Output;

impl Output for ParquetProducer {
  type Output = RecordBatch;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}
