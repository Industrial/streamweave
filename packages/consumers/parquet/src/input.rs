use crate::parquet_consumer::ParquetConsumer;
use arrow::record_batch::RecordBatch;
use futures::Stream;
use std::pin::Pin;
use streamweave_core::Input;

impl Input for ParquetConsumer {
  type Input = RecordBatch;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}
