use crate::consumers::parquet::parquet_consumer::ParquetConsumer;
use crate::input::Input;
use arrow::record_batch::RecordBatch;
use futures::Stream;
use std::pin::Pin;

impl Input for ParquetConsumer {
  type Input = RecordBatch;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}
