//! Parquet write node for StreamWeave graphs
//!
//! Writes data to Parquet files while passing data through. Takes RecordBatch as input, writes to Parquet,
//! and outputs the same data, enabling writing intermediate results while continuing processing.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::ParquetWriteTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;
use futures::Stream;
use std::path::PathBuf;
use std::pin::Pin;

/// Node that writes data to Parquet files while passing data through.
///
/// This node wraps `ParquetWriteTransformer` for use in graphs. It takes RecordBatch as input,
/// writes it to a Parquet file, and outputs the same data, enabling writing intermediate
/// results while continuing processing.
///
/// # Example
///
/// ```rust,no_run
/// use crate::graph::nodes::{ParquetWrite, TransformerNode};
/// use arrow::record_batch::RecordBatch;
///
/// let parquet_write = ParquetWrite::new("output.parquet");
/// let node = TransformerNode::from_transformer(
///     "parquet_write".to_string(),
///     parquet_write,
/// );
/// ```
pub struct ParquetWrite {
  /// The underlying Parquet write transformer
  transformer: ParquetWriteTransformer,
}

impl ParquetWrite {
  /// Creates a new `ParquetWrite` node for the specified file path.
  ///
  /// # Arguments
  ///
  /// * `path` - Path to the Parquet file to write.
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      transformer: ParquetWriteTransformer::new(path),
    }
  }

  /// Sets the error handling strategy for this node.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<RecordBatch>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }

  /// Sets the compression codec.
  ///
  /// # Arguments
  ///
  /// * `compression` - The compression codec to use.
  pub fn with_compression(
    mut self,
    compression: crate::transformers::ParquetWriteTransformerCompression,
  ) -> Self {
    self.transformer = self.transformer.with_compression(compression);
    self
  }

  /// Sets the maximum row group size.
  ///
  /// # Arguments
  ///
  /// * `size` - The maximum row group size.
  pub fn with_max_row_group_size(mut self, size: usize) -> Self {
    self.transformer = self.transformer.with_max_row_group_size(size);
    self
  }

  /// Sets the writer version.
  ///
  /// # Arguments
  ///
  /// * `version` - The writer version to use.
  pub fn with_writer_version(
    mut self,
    version: crate::transformers::ParquetWriteTransformerVersion,
  ) -> Self {
    self.transformer = self.transformer.with_writer_version(version);
    self
  }

  /// Returns the file path.
  #[must_use]
  pub fn path(&self) -> &PathBuf {
    self.transformer.path()
  }
}

impl Clone for ParquetWrite {
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl Input for ParquetWrite {
  type Input = RecordBatch;
  type InputStream = Pin<Box<dyn Stream<Item = RecordBatch> + Send>>;
}

impl Output for ParquetWrite {
  type Output = RecordBatch;
  type OutputStream = Pin<Box<dyn Stream<Item = RecordBatch> + Send>>;
}

#[async_trait]
impl Transformer for ParquetWrite {
  type InputPorts = (RecordBatch,);
  type OutputPorts = (RecordBatch,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<RecordBatch>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<RecordBatch> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<RecordBatch> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<RecordBatch>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<RecordBatch>) -> ErrorContext<RecordBatch> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}
